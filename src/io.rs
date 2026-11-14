//! Knuth, tex.web §§1–98, 103, 177–178, 241–246, 511–538. Detailed contracts: docs/semantics.md.

use crate::arithmetic::Scaled;
use crate::memory::{FIL_ORDER, FILLL_ORDER, MemPtr, Memory, NORMAL_ORDER};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// §2: Banner printed when TeX starts.
pub const BANNER: &str = "This is TeX, Version 3.141592653";
/// §11: Fixed-width 40-character name of the string pool file.
pub const POOL_NAME: &str = "TeXformats:TEX.POOL                     ";
/// §53: TANGLE checksum (`@$`) for the pinned `vendor/knuth/tex.web`.
pub const POOL_CHECKSUM: i32 = 504_454_778;
/// Pinned TANGLE-generated string pool bytes (`vendor/generated/tex.pool`).
pub const PINNED_TEX_POOL: &[u8] = include_bytes!("../vendor/generated/tex.pool");

/// §22: Symbolic ASCII code names.
pub const NULL_CODE: u8 = 0;
pub const CARRIAGE_RETURN: u8 = 13;
pub const INVALID_CODE: u8 = 127;

/// §§514, 520–521: Default system file areas and format name template.
pub const TEX_AREA: &str = "TeXinputs:";
pub const TEX_FONT_AREA: &str = "TeXfonts:";
pub const TEX_FORMAT_DEFAULT: &[u8; 20] = b"TeXformats:plain.fmt";
pub const FORMAT_DEFAULT_LENGTH: usize = 20;
pub const FORMAT_AREA_LENGTH: usize = 11;
pub const FORMAT_EXT_LENGTH: usize = 4;

/// §38: String number in `0..max_strings`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrNum(pub u16);

impl StrNum {
    pub const ZERO: Self = Self(0);
    pub const BUFFER_SIZE: Self = Self(256);
    pub const POOL_SIZE: Self = Self(257);
    pub const NUMBER_OF_STRINGS: Self = Self(258);
    pub const UNKNOWN: Self = Self(259);
    pub const ROMAN_SPEC: Self = Self(260);
    pub const EOF_TERMINAL: Self = Self(261);
    pub const BANG: Self = Self(262);
    pub const HUNDRED_ERRORS: Self = Self(263);
    pub const QUESTION_PROMPT: Self = Self(264);
    pub const YOU_WANT_TO_EDIT: Self = Self(265);
    pub const AT_LINE: Self = Self(266);
    pub const MENU_PROCEED: Self = Self(267);
    pub const MENU_RUN: Self = Self(268);
    pub const MENU_INSERT: Self = Self(269);
    pub const MENU_EDIT: Self = Self(270);
    pub const MENU_DELETE: Self = Self(271);
    pub const MENU_HELP_QUIT: Self = Self(272);
    pub const OK_ENTERING: Self = Self(273);
    pub const BATCHMODE: Self = Self(274);
    pub const NONSTOPMODE: Self = Self(275);
    pub const SCROLLMODE: Self = Self(276);
    pub const ELLIPSIS: Self = Self(277);
    pub const INSERT_PROMPT: Self = Self(278);
    pub const DELETED_HELP_1: Self = Self(279);
    pub const DELETED_HELP_0: Self = Self(280);
    pub const NO_HELP_1: Self = Self(281);
    pub const NO_HELP_0: Self = Self(282);
    pub const ALREADY_HELPED_3: Self = Self(283);
    pub const ALREADY_HELPED_1: Self = Self(284);
    pub const ALREADY_HELPED_0: Self = Self(285);
    pub const SPACE_LPAREN: Self = Self(286);
    pub const EMERGENCY_STOP: Self = Self(287);
    pub const CAPACITY_EXCEEDED: Self = Self(288);
    pub const WIZARD_1: Self = Self(289);
    pub const WIZARD_0: Self = Self(290);
    pub const THIS_CANT_HAPPEN: Self = Self(291);
    pub const IM_BROKEN: Self = Self(292);
    pub const CANT_GO_ON: Self = Self(293);
    pub const FAUX_PAS_1: Self = Self(294);
    pub const FAUX_PAS_0: Self = Self(295);
    pub const INTERRUPTION: Self = Self(296);
    pub const YOU_RANG_2: Self = Self(297);
    pub const YOU_RANG_1: Self = Self(298);
    pub const YOU_RANG_0: Self = Self(299);
    pub const MAIN_MEMORY_SIZE: Self = Self(300);
    pub const EMPTY: Self = Self(338);
    pub const COLON_PROMPT: Self = Self(568);
    pub const TEX_AREA_STR: Self = Self(784);
    pub const TEX_FONT_AREA_STR: Self = Self(785);
    pub const DOT_FMT: Self = Self(786);
    pub const INPUT_FILE_NAME: Self = Self(787);
    pub const CANT_FIND_FILE: Self = Self(788);
    pub const CANT_WRITE_FILE: Self = Self(789);
    pub const QUOTE_DOT: Self = Self(790);
    pub const DOT_TEX: Self = Self(791);
    pub const PLEASE_TYPE_ANOTHER: Self = Self(792);
    pub const ABORTED_FILE_ERROR: Self = Self(793);
    pub const DOT_DVI: Self = Self(794);
    pub const FILE_NAME_FOR_OUTPUT: Self = Self(795);
    pub const TEXPUT: Self = Self(796);
    pub const DOT_LOG: Self = Self(797);
    pub const DOUBLE_STAR: Self = Self(798);
    pub const TRANSCRIPT_FILE_NAME: Self = Self(799);
    pub const TWO_SPACES: Self = Self(800);
    pub const INITEX_IDENT: Self = Self(1257);

    pub const fn from_ascii(byte: u8) -> Self {
        Self(byte as u16)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// Resolve a WEB string literal to its exact TANGLE `StrNum` (`0..=255` for 1-byte ASCII, `256..=1300` in `TEX.POOL`).
pub fn pool_str(lit: &str) -> StrNum {
    if lit.len() == 1 {
        return StrNum::from_ascii(lit.as_bytes()[0]);
    }
    static POOL_INDEX: OnceLock<BTreeMap<&'static str, StrNum>> = OnceLock::new();
    let map = POOL_INDEX.get_or_init(|| {
        let text = std::str::from_utf8(PINNED_TEX_POOL).expect("valid ASCII pool");
        let mut m = BTreeMap::new();
        for (idx, line) in text.lines().enumerate() {
            if line.starts_with('*') {
                break;
            }
            let body = &line[2..];
            m.insert(body, StrNum((256 + idx) as u16));
        }
        m
    });
    *map.get(lit)
        .unwrap_or_else(|| panic!("string literal {lit:?} not found in pinned TEX.POOL"))
}

/// §§6, 35, 81: Control-transfer target when TeX aborts or terminates early.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Termination {
    /// Label `end_of_TEX = 9998`, reached via `jump_out` (§81).
    JumpOut,
    /// Label `final_end = 9999`, reached before error recovery is active (§35).
    FinalEnd,
}

/// §§11–14, 111, 290, 522: Capacity parameters and consistency checks for I/O, strings, and tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IoConfig {
    pub buf_size: usize,
    pub error_line: usize,
    pub half_error_line: usize,
    pub max_print_line: usize,
    pub stack_size: usize,
    pub max_in_open: usize,
    pub param_size: usize,
    pub nest_size: usize,
    pub max_strings: usize,
    pub string_vacancies: usize,
    pub pool_size: usize,
    pub file_name_size: usize,
    pub dvi_buf_size: usize,
    pub mem_bot: usize,
    pub mem_top: usize,
    pub hash_size: usize,
    pub hash_prime: usize,
    pub save_size: usize,
}

impl Default for IoConfig {
    fn default() -> Self {
        Self {
            buf_size: 500,
            error_line: 72,
            half_error_line: 42,
            max_print_line: 79,
            stack_size: 200,
            max_in_open: 6,
            param_size: 60,
            nest_size: 40,
            max_strings: 3000,
            string_vacancies: 8000,
            pool_size: 32000,
            file_name_size: 40,
            dvi_buf_size: 800,
            mem_bot: 0,
            mem_top: 30000,
            hash_size: 2100,
            hash_prime: 1777,
            save_size: 600,
        }
    }
}

impl IoConfig {
    /// TRIP profile parameters from `tripman.tex` Appendix A (`testing.md`).
    pub fn trip() -> Self {
        Self {
            mem_bot: 1,
            mem_top: 3000,
            error_line: 64,
            half_error_line: 32,
            max_print_line: 72,
            ..Self::default()
        }
    }

    /// §§13–14, 111, 290, 522: Evaluate `bad` constant checks in source order.
    pub fn check_constants(&self) -> Result<(), u8> {
        let mut bad = 0_u8;
        if self.half_error_line < 30 || self.half_error_line + 15 > self.error_line {
            bad = 1;
        }
        if self.max_print_line < 60 {
            bad = 2;
        }
        if self.dvi_buf_size == 0 || self.dvi_buf_size % 8 != 0 {
            bad = 3;
        }
        if self.mem_bot + 1100 > self.mem_top {
            bad = 4;
        }
        if self.hash_prime > self.hash_size {
            bad = 5;
        }
        if self.max_in_open >= 128 {
            bad = 6;
        }
        if self.mem_top < 256 + 11 {
            bad = 7;
        }
        if self.save_size > 65535 || self.max_strings > 65535 || self.max_strings < 256 {
            bad = 17;
        }
        if self.buf_size == 0 || self.buf_size > 65535 {
            bad = 18;
        }
        // §290: cs_token_flag (4095) + undefined_control_sequence (781 + hash_size) > max_halfword (65535).
        if 4095usize.saturating_add(781).saturating_add(self.hash_size) > 65535 {
            bad = 21;
        }
        if FORMAT_DEFAULT_LENGTH > self.file_name_size {
            bad = 31;
        }
        if bad == 0 { Ok(()) } else { Err(bad) }
    }
}

/// §§17–24: `xchr` and `xord` character translation tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharTranslator {
    xchr: [u8; 256],
    xord: [u8; 256],
}

impl Default for CharTranslator {
    fn default() -> Self {
        Self::pristine()
    }
}

impl CharTranslator {
    /// §§21, 23–24: Pristine WEB initialization blanking `xchr[0..31]` and `xchr[127..255]`.
    pub fn pristine() -> Self {
        let mut xchr = [b' '; 256];
        for (i, slot) in xchr.iter_mut().enumerate().take(127).skip(32) {
            *slot = i as u8;
        }
        Self::from_xchr(xchr)
    }

    /// §23 permissive adaptation (`xchr[i] = chr(i)`), with §24's `xord` inversion.
    pub fn identity_eight_bit() -> Self {
        let mut xchr = [0_u8; 256];
        for (i, slot) in xchr.iter_mut().enumerate() {
            *slot = i as u8;
        }
        Self::from_xchr(xchr)
    }

    /// §24: Compute `xord` from `xchr` using the exact source loop order.
    pub fn from_xchr(xchr: [u8; 256]) -> Self {
        let mut xord = [INVALID_CODE; 256];
        for i in 128..=255_usize {
            xord[usize::from(xchr[i])] = i as u8;
        }
        for i in 0..=126_usize {
            xord[usize::from(xchr[i])] = i as u8;
        }
        Self { xchr, xord }
    }

    pub const fn xchr(&self, code: u8) -> u8 {
        self.xchr[code as usize]
    }

    pub const fn xord(&self, ext: u8) -> u8 {
        self.xord[ext as usize]
    }
}

/// §49: Whether `k` is a printable visible ASCII character (`" " ..= "~"`).
pub const fn is_printable_ascii(k: u8) -> bool {
    k >= b' ' && k <= b'~'
}

/// §§25, 30–33: Pascal text-file input stream (`alpha_file` opened for input).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AlphaFileIn {
    bytes: Vec<u8>,
    pos: usize,
    started: bool,
    is_open: bool,
}

impl AlphaFileIn {
    /// Open an ordinary text file (`reset(f, name, '/O')`), loading the first character.
    pub fn new_file(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            pos: 0,
            started: true,
            is_open: true,
        }
    }

    /// §33: Open terminal input (`reset(term_in, 'TTY:', '/O/I')`), suppressing the first `get`.
    pub fn new_terminal(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            pos: 0,
            started: false,
            is_open: true,
        }
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// §28: `a_close`. Closing a file that is not open does not error.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn eof(&self) -> bool {
        !self.is_open || (self.started && self.pos >= self.bytes.len())
    }

    /// §30: `eoln` is true at LF (`\n`) or when `eof` occurs in the middle of a line.
    pub fn eoln(&self) -> bool {
        self.eof() || (self.started && self.bytes[self.pos] == b'\n')
    }

    /// Pascal `f^`: current character buffer, or `' '` at `eoln`/`eof`.
    pub fn current(&self) -> u8 {
        if !self.is_open || !self.started || self.pos >= self.bytes.len() {
            b' '
        } else {
            let b = self.bytes[self.pos];
            if b == b'\n' { b' ' } else { b }
        }
    }

    /// Pascal `get(f)`.
    pub fn get(&mut self) {
        if !self.is_open {
            return;
        }
        if !self.started {
            self.started = true;
        } else if self.pos < self.bytes.len() {
            self.pos += 1;
        }
    }

    /// Pascal `read(f, ch)`.
    pub fn read_char(&mut self) -> u8 {
        let c = self.current();
        self.get();
        c
    }

    /// Pascal `read_ln(f)`.
    pub fn read_ln(&mut self) {
        while !self.eoln() {
            self.get();
        }
        if !self.eof() {
            self.get();
        }
    }
}

/// §§25, 27–28: Binary input stream (`byte_file` / `word_file` opened for input).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ByteFileIn {
    bytes: Vec<u8>,
    pos: usize,
    is_open: bool,
}

impl ByteFileIn {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            pos: 0,
            is_open: true,
        }
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn eof(&self) -> bool {
        !self.is_open || self.pos >= self.bytes.len()
    }

    pub fn current(&self) -> Option<u8> {
        if self.eof() {
            None
        } else {
            Some(self.bytes[self.pos])
        }
    }

    pub fn get(&mut self) {
        if self.is_open && self.pos < self.bytes.len() {
            self.pos += 1;
        }
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        let b = self.current()?;
        self.get();
        Some(b)
    }
}

/// §241: Initial date and time supplied by the host (`fix_date_and_time`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    pub sys_time: i32,
    pub sys_day: i32,
    pub sys_month: i32,
    pub sys_year: i32,
}

impl Default for Clock {
    /// §241: Noon on 4 July 1776 ("self-evident truths").
    fn default() -> Self {
        Self {
            sys_time: 12 * 60,
            sys_day: 4,
            sys_month: 7,
            sys_year: 1776,
        }
    }
}

/// Deterministic in-memory host fixture for terminal I/O, named files, and clock (§§25–37, 241, 511–538).
#[derive(Clone, Debug, Default)]
pub struct HostFixture {
    pub term_in: AlphaFileIn,
    pub term_out: Vec<u8>,
    pub log_file: Vec<u8>,
    pub write_files: [Vec<u8>; 16],
    pub dvi_file: Vec<u8>,
    pub clock: Clock,
    pub update_terminal_calls: usize,
    pub clear_terminal_calls: usize,
    pub wake_up_terminal_calls: usize,
    files: BTreeMap<String, Vec<u8>>,
    unwritable_names: BTreeSet<String>,
    clear_terminal_drops_input: bool,
}

impl HostFixture {
    pub fn with_pinned_pool(terminal_input: &[u8]) -> Self {
        let mut host = Self {
            term_in: AlphaFileIn::new_terminal(terminal_input.to_vec()),
            ..Self::default()
        };
        host.insert_file(POOL_NAME.trim_end(), PINNED_TEX_POOL.to_vec());
        host
    }

    pub fn set_terminal_input(&mut self, bytes: &[u8]) {
        self.term_in = AlphaFileIn::new_terminal(bytes.to_vec());
    }

    pub fn set_clear_terminal_drops_input(&mut self, drop_input: bool) {
        self.clear_terminal_drops_input = drop_input;
    }

    pub fn insert_file(&mut self, name: &str, contents: impl Into<Vec<u8>>) {
        self.files
            .insert(name.trim_end_matches(' ').to_owned(), contents.into());
    }

    pub fn remove_file(&mut self, name: &str) {
        self.files.remove(name.trim_end_matches(' '));
    }

    pub fn mark_unwritable(&mut self, name: &str) {
        self.unwritable_names
            .insert(name.trim_end_matches(' ').to_owned());
    }

    pub fn allow_write(&mut self, name: &str) {
        self.unwritable_names.remove(name.trim_end_matches(' '));
    }

    pub fn file_bytes(&self, name: &str) -> Option<&[u8]> {
        self.files
            .get(name.trim_end_matches(' '))
            .map(Vec::as_slice)
    }

    /// §27: `a_open_in` using trimmed `name_of_file` ("Blank spaces that might appear in name are ignored").
    pub fn a_open_in(&self, name_of_file: &str) -> Option<AlphaFileIn> {
        let key = name_of_file.trim_end_matches(' ');
        self.files.get(key).cloned().map(AlphaFileIn::new_file)
    }

    /// §27: `b_open_in` / `w_open_in`.
    pub fn b_open_in(&self, name_of_file: &str) -> Option<ByteFileIn> {
        let key = name_of_file.trim_end_matches(' ');
        self.files.get(key).cloned().map(ByteFileIn::new)
    }

    /// §27: `a_open_out` / `b_open_out` / `w_open_out`.
    pub fn can_open_out(&self, name_of_file: &str) -> bool {
        let key = name_of_file.trim_end_matches(' ');
        !key.is_empty() && !self.unwritable_names.contains(key)
    }
}

/// §54: Destination of `print` commands (`0..=21`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selector {
    Write(u8),
    NoPrint,
    TermOnly,
    LogOnly,
    TermAndLog,
    Pseudo,
    NewString,
}

impl Selector {
    pub const fn code(self) -> u8 {
        match self {
            Self::Write(n) => n,
            Self::NoPrint => 16,
            Self::TermOnly => 17,
            Self::LogOnly => 18,
            Self::TermAndLog => 19,
            Self::Pseudo => 20,
            Self::NewString => 21,
        }
    }

    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0..=15 => Some(Self::Write(code)),
            16 => Some(Self::NoPrint),
            17 => Some(Self::TermOnly),
            18 => Some(Self::LogOnly),
            19 => Some(Self::TermAndLog),
            20 => Some(Self::Pseudo),
            21 => Some(Self::NewString),
            _ => None,
        }
    }

    pub const fn is_odd(self) -> bool {
        self.code() % 2 == 1
    }

    pub fn decr(&mut self) {
        if let Some(prev) = Self::from_code(self.code().saturating_sub(1)) {
            *self = prev;
        }
    }

    pub fn incr(&mut self) {
        if let Some(next) = Self::from_code(self.code().saturating_add(1)) {
            *self = next;
        }
    }
}

/// §73: User interaction level (`0..=3`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Interaction {
    BatchMode = 0,
    NonstopMode = 1,
    ScrollMode = 2,
    ErrorStopMode = 3,
}

impl Interaction {
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::BatchMode),
            1 => Some(Self::NonstopMode),
            2 => Some(Self::ScrollMode),
            3 => Some(Self::ErrorStopMode),
            _ => None,
        }
    }
}

/// §76: Worst error severity recorded so far (`0..=3`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum History {
    Spotless = 0,
    WarningIssued = 1,
    ErrorMessageIssued = 2,
    FatalErrorStop = 3,
}

/// §§300–302: Subset of TeX's `in_state_record` used by M3 I/O, errors, and file names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InStateRecord {
    pub state_field: u8,
    pub index_field: u8,
    pub start_field: usize,
    pub loc_field: usize,
    pub limit_field: usize,
    pub name_field: StrNum,
}

/// §78: Callbacks for syntactic/token subsystems (`M4`) invoked by `error` (§§82–90) and `prompt_file_name` (§530).
pub trait ErrorHooks {
    fn show_context(&mut self, _io: &mut IoState) {}
    fn clear_for_error_prompt(&mut self, io: &mut IoState) {
        io.clear_for_error_prompt_default();
    }
    fn get_token(&mut self, _io: &mut IoState) -> Result<(), Termination> {
        Ok(())
    }
    fn begin_file_reading(&mut self, io: &mut IoState) -> Result<(), Termination> {
        let mut hooks = NoopErrorHooks;
        io.begin_file_reading_with_hooks(&mut hooks)
    }
    fn end_file_reading(&mut self, io: &mut IoState) {
        io.end_file_reading_default();
    }
    fn give_err_help(&mut self, _io: &mut IoState) {}
}

/// Default no-op syntactic hook implementation before M4 is attached.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopErrorHooks;

impl ErrorHooks for NoopErrorHooks {}

/// §§1–98, 103, 241–246, 511–538: State for character translation, buffers, string pool, printing, errors, and file names.
#[derive(Clone, Debug)]
pub struct IoState {
    pub config: IoConfig,
    pub chars: CharTranslator,
    pub host: HostFixture,

    // §§30, 36: Line buffer and pointers.
    pub buffer: Vec<u8>,
    pub first: usize,
    pub last: usize,
    pub max_buf_stack: usize,
    pub cur_input: InStateRecord,
    pub input_stack: Vec<InStateRecord>,
    pub input_ptr: usize,
    pub max_in_stack: usize,
    pub base_ptr: usize,
    pub in_open: usize,
    pub open_parens: usize,
    pub input_file: Vec<AlphaFileIn>,
    pub line: i32,
    pub line_stack: Vec<i32>,

    // §§38–46: String pool.
    pub str_pool: Vec<u8>,
    pub str_start: Vec<usize>,
    pub pool_ptr: usize,
    pub str_ptr: usize,
    pub init_pool_ptr: usize,
    pub init_str_ptr: usize,

    // §§54–71: Printing state.
    pub selector: Selector,
    pub dig: [u8; 23],
    pub tally: i32,
    pub term_offset: usize,
    pub file_offset: usize,
    pub trick_buf: Vec<u8>,
    pub trick_count: i32,
    pub first_count: i32,
    pub old_setting: Selector,

    // §§72–98: Error and interaction state.
    pub interaction: Interaction,
    pub deletions_allowed: bool,
    pub set_box_allowed: bool,
    pub history: History,
    pub error_count: i8,
    pub help_line: [StrNum; 6],
    pub help_ptr: usize,
    pub use_err_help: bool,
    pub interrupt: i32,
    pub ok_to_interrupt: bool,
    pub cur_tok: i32,
    pub cur_cmd: u8,
    pub cur_chr: i32,
    pub align_state: i32,

    // §§240–246: Parameters used by printing/I/O.
    pub escape_char: i32,
    pub end_line_char: i32,
    pub new_line_char: i32,
    pub tracing_online: i32,
    pub pausing: i32,
    pub sys_time: i32,
    pub sys_day: i32,
    pub sys_month: i32,
    pub sys_year: i32,
    pub time: i32,
    pub day: i32,
    pub month: i32,
    pub year: i32,

    // §§26, 511–538, 1299: File names and format ident.
    pub name_of_file: Vec<u8>,
    pub name_length: usize,
    pub cur_name: StrNum,
    pub cur_area: StrNum,
    pub cur_ext: StrNum,
    pub area_delimiter: usize,
    pub ext_delimiter: usize,
    pub tex_format_default: [u8; FORMAT_DEFAULT_LENGTH],
    pub name_in_progress: bool,
    pub job_name: StrNum,
    pub log_opened: bool,
    pub log_name: StrNum,
    pub output_file_name: StrNum,
    pub format_ident: StrNum,
}

impl IoState {
    /// Create a new I/O and string subsystem after validating `IoConfig::check_constants`.
    pub fn new(config: IoConfig, host: HostFixture) -> Result<Self, u8> {
        config.check_constants()?;
        let mut input_stack =
            vec![InStateRecord::default(); config.stack_size.max(config.buf_size).max(8) + 1];
        let cur_input = InStateRecord {
            state_field: 33, // new_line
            index_field: 0,
            start_field: 1,
            loc_field: 1,
            limit_field: 0,
            name_field: StrNum::ZERO,
        };
        input_stack[0] = cur_input;
        Ok(Self {
            config,
            chars: CharTranslator::pristine(),
            host,
            buffer: vec![0; config.buf_size + 1],
            first: 1,
            last: 1,
            max_buf_stack: 0,
            cur_input,
            input_stack,
            input_ptr: 0,
            max_in_stack: 0,
            base_ptr: 0,
            in_open: 0,
            open_parens: 0,
            input_file: vec![AlphaFileIn::default(); config.max_in_open + 1],
            line: 0,
            line_stack: vec![0; config.max_in_open + 1],
            str_pool: vec![0; config.pool_size + 1],
            str_start: vec![0; config.max_strings + 1],
            pool_ptr: 0,
            str_ptr: 0,
            init_pool_ptr: 0,
            init_str_ptr: 0,
            selector: Selector::TermOnly,
            dig: [0; 23],
            tally: 0,
            term_offset: 0,
            file_offset: 0,
            trick_buf: vec![0; config.error_line + 1],
            trick_count: 0,
            first_count: 0,
            old_setting: Selector::TermOnly,
            interaction: Interaction::ErrorStopMode,
            deletions_allowed: true,
            set_box_allowed: true,
            history: History::FatalErrorStop,
            error_count: 0,
            help_line: [StrNum::ZERO; 6],
            help_ptr: 0,
            use_err_help: false,
            interrupt: 0,
            ok_to_interrupt: true,
            cur_tok: 0,
            cur_cmd: 0,
            cur_chr: 0,
            align_state: 1_000_000,
            escape_char: i32::from(b'\\'),
            end_line_char: i32::from(CARRIAGE_RETURN),
            new_line_char: -1,
            tracing_online: 0,
            pausing: 0,
            sys_time: 0,
            sys_day: 0,
            sys_month: 0,
            sys_year: 0,
            time: 0,
            day: 0,
            month: 0,
            year: 0,
            name_of_file: vec![b' '; config.file_name_size],
            name_length: 0,
            cur_name: StrNum::ZERO,
            cur_area: StrNum::ZERO,
            cur_ext: StrNum::ZERO,
            area_delimiter: 0,
            ext_delimiter: 0,
            tex_format_default: *TEX_FORMAT_DEFAULT,
            name_in_progress: false,
            job_name: StrNum::ZERO,
            log_opened: false,
            log_name: StrNum::ZERO,
            output_file_name: StrNum::ZERO,
            format_ident: StrNum::ZERO,
        })
    }

    /// §241: `fix_date_and_time`.
    pub fn fix_date_and_time(&mut self) {
        self.sys_time = self.host.clock.sys_time;
        self.sys_day = self.host.clock.sys_day;
        self.sys_month = self.host.clock.sys_month;
        self.sys_year = self.host.clock.sys_year;
        self.time = self.sys_time;
        self.day = self.sys_day;
        self.month = self.sys_month;
        self.year = self.sys_year;
    }

    /// §75: Initialize `selector` based on `interaction`.
    pub fn init_selector_from_interaction(&mut self) {
        if self.interaction == Interaction::BatchMode {
            self.selector = Selector::NoPrint;
        } else {
            self.selector = Selector::TermOnly;
        }
    }

    /// §34: `update_terminal`.
    pub fn update_terminal(&mut self) {
        self.host.update_terminal_calls += 1;
    }

    /// §34: `clear_terminal`.
    pub fn clear_terminal(&mut self) {
        self.host.clear_terminal_calls += 1;
        if self.host.clear_terminal_drops_input {
            self.host.term_in = AlphaFileIn::new_terminal(Vec::new());
        }
    }

    /// §34: `wake_up_terminal`.
    pub fn wake_up_terminal(&mut self) {
        self.host.wake_up_terminal_calls += 1;
    }

    /// §40: `length(s)`.
    pub fn length(&self, s: StrNum) -> usize {
        let idx = usize::from(s.0);
        if idx < self.str_ptr {
            self.str_start[idx + 1] - self.str_start[idx]
        } else {
            0
        }
    }

    /// §41: `cur_length`.
    pub fn cur_length(&self) -> usize {
        self.pool_ptr - self.str_start[self.str_ptr]
    }

    /// Bytes of finalized string `s` in `str_pool`.
    pub fn str_bytes(&self, s: StrNum) -> &[u8] {
        let idx = usize::from(s.0);
        if idx < self.str_ptr {
            &self.str_pool[self.str_start[idx]..self.str_start[idx + 1]]
        } else {
            &[]
        }
    }

    /// §42: `append_char`. Caller must check `str_room` first.
    pub fn append_char(&mut self, c: u8) {
        self.str_pool[self.pool_ptr] = c;
        self.pool_ptr += 1;
    }

    /// §42: `flush_char`.
    pub fn flush_char(&mut self) {
        self.pool_ptr -= 1;
    }

    /// §42: `str_room(l)`.
    pub fn str_room(&mut self, l: usize) -> Result<(), Termination> {
        let mut hooks = NoopErrorHooks;
        self.str_room_with_hooks(l, &mut hooks)
    }

    pub fn str_room_with_hooks(
        &mut self,
        l: usize,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        if self.pool_ptr + l > self.config.pool_size {
            let n = (self.config.pool_size - self.init_pool_ptr) as i32;
            return Err(self.overflow_with_hooks(StrNum::POOL_SIZE, n, hooks));
        }
        Ok(())
    }

    /// §43: `make_string`.
    pub fn make_string(&mut self) -> Result<StrNum, Termination> {
        let mut hooks = NoopErrorHooks;
        self.make_string_with_hooks(&mut hooks)
    }

    pub fn make_string_with_hooks(
        &mut self,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<StrNum, Termination> {
        if self.str_ptr == self.config.max_strings {
            let n = (self.config.max_strings - self.init_str_ptr) as i32;
            return Err(self.overflow_with_hooks(StrNum::NUMBER_OF_STRINGS, n, hooks));
        }
        self.str_ptr += 1;
        self.str_start[self.str_ptr] = self.pool_ptr;
        Ok(StrNum((self.str_ptr - 1) as u16))
    }

    /// §44: `flush_string`.
    pub fn flush_string(&mut self) {
        self.str_ptr -= 1;
        self.pool_ptr = self.str_start[self.str_ptr];
    }

    /// §45: `str_eq_buf(s, k)`.
    pub fn str_eq_buf(&self, s: StrNum, mut k: usize) -> bool {
        let idx = usize::from(s.0);
        let mut j = self.str_start[idx];
        let end = self.str_start[idx + 1];
        while j < end {
            if k >= self.buffer.len() || self.str_pool[j] != self.buffer[k] {
                return false;
            }
            j += 1;
            k += 1;
        }
        true
    }

    /// §46: `str_eq_str(s, t)`.
    pub fn str_eq_str(&self, s: StrNum, t: StrNum) -> bool {
        if self.length(s) != self.length(t) {
            return false;
        }
        let s_idx = usize::from(s.0);
        let t_idx = usize::from(t.0);
        let mut j = self.str_start[s_idx];
        let mut k = self.str_start[t_idx];
        let end = self.str_start[s_idx + 1];
        while j < end {
            if self.str_pool[j] != self.str_pool[k] {
                return false;
            }
            j += 1;
            k += 1;
        }
        true
    }

    /// §§47–53: `get_strings_started`. Initializes strings `0..255` and loads `TEX.POOL`.
    pub fn get_strings_started(&mut self) -> Result<bool, Termination> {
        self.pool_ptr = 0;
        self.str_ptr = 0;
        self.str_start[0] = 0;

        // §§48–49: Make the first 256 strings.
        for k in 0..=255_u16 {
            let b = k as u8;
            if !is_printable_ascii(b) {
                self.append_char(b'^');
                self.append_char(b'^');
                if k < 64 {
                    self.append_char(b + 64);
                } else if k < 128 {
                    self.append_char(b - 64);
                } else {
                    let hi = b / 16;
                    self.append_char(if hi < 10 { hi + b'0' } else { hi - 10 + b'a' });
                    let lo = b % 16;
                    self.append_char(if lo < 10 { lo + b'0' } else { lo - 10 + b'a' });
                }
            } else {
                self.append_char(b);
            }
            self.make_string()?;
        }

        // §51: Read the other strings from the TEX.POOL file.
        self.set_name_of_file_from_bytes(POOL_NAME.as_bytes());
        let name_str = String::from_utf8_lossy(&self.name_of_file).into_owned();
        let Some(mut pool_file) = self.host.a_open_in(&name_str) else {
            self.bad_pool(b"! I can't read TEX.POOL.");
            return Ok(false);
        };

        let mut checksum_checked = false;
        while !checksum_checked {
            if pool_file.eof() {
                pool_file.close();
                self.bad_pool(b"! TEX.POOL has no check sum.");
                return Ok(false);
            }
            let m = pool_file.read_char();
            let mut n = pool_file.read_char();
            if m == b'*' {
                // §53: Check the pool check sum.
                let mut a = 0_i32;
                let mut k = 1_usize;
                loop {
                    let xn = self.chars.xord(n);
                    if !xn.is_ascii_digit() {
                        pool_file.close();
                        self.bad_pool(b"! TEX.POOL check sum doesn't have nine digits.");
                        return Ok(false);
                    }
                    a = 10 * a + i32::from(xn - b'0');
                    if k == 9 {
                        break;
                    }
                    k += 1;
                    n = pool_file.read_char();
                }
                if a != POOL_CHECKSUM {
                    pool_file.close();
                    self.bad_pool(b"! TEX.POOL doesn't match; TANGLE me again.");
                    return Ok(false);
                }
                checksum_checked = true;
            } else {
                // §52: Read one string.
                let xm = self.chars.xord(m);
                let xn = self.chars.xord(n);
                if !xm.is_ascii_digit() || !xn.is_ascii_digit() {
                    pool_file.close();
                    self.bad_pool(b"! TEX.POOL line doesn't begin with two digits.");
                    return Ok(false);
                }
                let l = usize::from(xm - b'0') * 10 + usize::from(xn - b'0');
                if self.pool_ptr + l + self.config.string_vacancies > self.config.pool_size {
                    pool_file.close();
                    self.bad_pool(b"! You have to increase POOLSIZE.");
                    return Ok(false);
                }
                for _ in 0..l {
                    let ch = if pool_file.eoln() {
                        b' '
                    } else {
                        pool_file.read_char()
                    };
                    let ord = self.chars.xord(ch);
                    self.append_char(ord);
                }
                pool_file.read_ln();
                self.make_string()?;
            }
        }
        pool_file.close();
        self.init_str_ptr = self.str_ptr;
        self.init_pool_ptr = self.pool_ptr;
        Ok(true)
    }

    fn bad_pool(&mut self, msg: &[u8]) {
        self.wake_up_terminal();
        self.wterm_ln(msg);
    }

    fn set_name_of_file_from_bytes(&mut self, src: &[u8]) {
        self.name_of_file.fill(b' ');
        let count = src.len().min(self.config.file_name_size);
        self.name_of_file[..count].copy_from_slice(&src[..count]);
    }

    /// §§31, 35: `input_ln(f, bypass_eoln)`.
    pub fn input_ln(
        &mut self,
        f: &mut AlphaFileIn,
        bypass_eoln: bool,
    ) -> Result<bool, Termination> {
        let mut hooks = NoopErrorHooks;
        self.input_ln_with_hooks(f, bypass_eoln, &mut hooks)
    }

    pub fn input_ln_with_hooks(
        &mut self,
        f: &mut AlphaFileIn,
        bypass_eoln: bool,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<bool, Termination> {
        if bypass_eoln && !f.eof() {
            f.get();
        }
        self.last = self.first;
        if f.eof() {
            return Ok(false);
        }
        let mut last_nonblank = self.first;
        while !f.eoln() {
            if self.last >= self.max_buf_stack {
                self.max_buf_stack = self.last + 1;
                if self.max_buf_stack == self.config.buf_size {
                    // §35: Report overflow of the input buffer, and abort.
                    if self.format_ident == StrNum::ZERO {
                        self.wterm_ln(b"Buffer size exceeded!");
                        return Err(Termination::FinalEnd);
                    }
                    self.cur_input.loc_field = self.first;
                    self.cur_input.limit_field = self.last.saturating_sub(1);
                    return Err(self.overflow_with_hooks(
                        StrNum::BUFFER_SIZE,
                        self.config.buf_size as i32,
                        hooks,
                    ));
                }
            }
            let raw_ch = f.current();
            self.buffer[self.last] = self.chars.xord(raw_ch);
            f.get();
            self.last += 1;
            if self.buffer[self.last - 1] != b' ' {
                last_nonblank = self.last;
            }
        }
        self.last = last_nonblank;
        Ok(true)
    }

    /// §37: `init_terminal`.
    pub fn init_terminal(&mut self) -> Result<bool, Termination> {
        loop {
            self.wake_up_terminal();
            self.wterm(b"**");
            self.update_terminal();
            let mut term_in = std::mem::take(&mut self.host.term_in);
            let read_res = self.input_ln(&mut term_in, true);
            self.host.term_in = term_in;
            if !read_res? {
                self.wterm_cr();
                self.wterm(b"! End of file on the terminal... why?");
                return Ok(false);
            }
            self.cur_input.loc_field = self.first;
            while self.cur_input.loc_field < self.last
                && self.buffer[self.cur_input.loc_field] == b' '
            {
                self.cur_input.loc_field += 1;
            }
            if self.cur_input.loc_field < self.last {
                return Ok(true);
            }
            self.wterm_ln(b"Please type the name of your input file.");
        }
    }

    fn wterm(&mut self, bytes: &[u8]) {
        self.host.term_out.extend_from_slice(bytes);
    }

    fn wterm_cr(&mut self) {
        self.host.term_out.push(b'\n');
    }

    fn wterm_ln(&mut self, bytes: &[u8]) {
        self.wterm(bytes);
        self.wterm_cr();
    }

    fn wlog(&mut self, bytes: &[u8]) {
        self.host.log_file.extend_from_slice(bytes);
    }

    fn wlog_cr(&mut self) {
        self.host.log_file.push(b'\n');
    }

    /// §57: `print_ln`. Does not affect `tally`.
    pub fn print_ln(&mut self) {
        match self.selector {
            Selector::TermAndLog => {
                self.wterm_cr();
                self.wlog_cr();
                self.term_offset = 0;
                self.file_offset = 0;
            }
            Selector::LogOnly => {
                self.wlog_cr();
                self.file_offset = 0;
            }
            Selector::TermOnly => {
                self.wterm_cr();
                self.term_offset = 0;
            }
            Selector::NoPrint | Selector::Pseudo | Selector::NewString => {}
            Selector::Write(stream) => {
                self.host.write_files[usize::from(stream)].push(b'\n');
            }
        }
    }

    /// §58: `print_char`.
    pub fn print_char(&mut self, s: u8) {
        if i32::from(s) == self.new_line_char && self.selector.code() < Selector::Pseudo.code() {
            self.print_ln();
            return;
        }
        let ext = self.chars.xchr(s);
        match self.selector {
            Selector::TermAndLog => {
                self.host.term_out.push(ext);
                self.host.log_file.push(ext);
                self.term_offset += 1;
                self.file_offset += 1;
                if self.term_offset == self.config.max_print_line {
                    self.wterm_cr();
                    self.term_offset = 0;
                }
                if self.file_offset == self.config.max_print_line {
                    self.wlog_cr();
                    self.file_offset = 0;
                }
            }
            Selector::LogOnly => {
                self.host.log_file.push(ext);
                self.file_offset += 1;
                if self.file_offset == self.config.max_print_line {
                    self.print_ln();
                }
            }
            Selector::TermOnly => {
                self.host.term_out.push(ext);
                self.term_offset += 1;
                if self.term_offset == self.config.max_print_line {
                    self.print_ln();
                }
            }
            Selector::NoPrint => {}
            Selector::Pseudo => {
                if self.tally < self.trick_count {
                    let idx = (self.tally as usize) % self.config.error_line;
                    self.trick_buf[idx] = s;
                }
            }
            Selector::NewString => {
                if self.pool_ptr < self.config.pool_size {
                    self.append_char(s);
                }
            }
            Selector::Write(stream) => {
                self.host.write_files[usize::from(stream)].push(ext);
            }
        }
        self.tally += 1;
    }

    /// §59: `print(s)`. Accepts any signed integer to preserve `s < 0` and `s >= str_ptr` checks.
    pub fn print(&mut self, mut s: i32) {
        if s >= self.str_ptr as i32 {
            s = i32::from(StrNum::UNKNOWN.0);
        } else if s < 256 {
            if s < 0 {
                s = i32::from(StrNum::UNKNOWN.0);
            } else {
                if self.selector.code() > Selector::Pseudo.code() {
                    self.print_char(s as u8);
                    return;
                }
                if s == self.new_line_char && self.selector.code() < Selector::Pseudo.code() {
                    self.print_ln();
                    return;
                }
                let nl = self.new_line_char;
                self.new_line_char = -1;
                let idx = s as usize;
                let mut j = self.str_start[idx];
                let end = self.str_start[idx + 1];
                while j < end {
                    let ch = self.str_pool[j];
                    self.print_char(ch);
                    j += 1;
                }
                self.new_line_char = nl;
                return;
            }
        }
        let idx = s as usize;
        if idx < self.str_ptr {
            let mut j = self.str_start[idx];
            let end = self.str_start[idx + 1];
            while j < end {
                let ch = self.str_pool[j];
                self.print_char(ch);
                j += 1;
            }
        } else {
            for &ch in b"???" {
                self.print_char(ch);
            }
        }
    }

    /// §59 convenience wrapper for `StrNum`.
    pub fn print_str(&mut self, s: StrNum) {
        self.print(i32::from(s.0));
    }

    /// §60: `slow_print(s)`.
    pub fn slow_print(&mut self, s: i32) {
        if s >= self.str_ptr as i32 || s < 256 {
            self.print(s);
        } else {
            let idx = s as usize;
            let mut j = self.str_start[idx];
            let end = self.str_start[idx + 1];
            while j < end {
                let ch = self.str_pool[j];
                self.print(i32::from(ch));
                j += 1;
            }
        }
    }

    /// §61: Print the initial banner headline to `term_out`.
    pub fn print_banner_line(&mut self) {
        self.wterm(BANNER.as_bytes());
        if self.format_ident == StrNum::ZERO {
            self.wterm_ln(b" (no format preloaded)");
        } else {
            self.slow_print(i32::from(self.format_ident.0));
            self.print_ln();
        }
        self.update_terminal();
    }

    /// §62: `print_nl(s)`.
    pub fn print_nl(&mut self, s: StrNum) {
        if (self.term_offset > 0 && self.selector.is_odd())
            || (self.file_offset > 0 && self.selector.code() >= Selector::LogOnly.code())
        {
            self.print_ln();
        }
        self.print_str(s);
    }

    /// §63: `print_esc(s)`.
    pub fn print_esc(&mut self, s: StrNum) {
        let c = self.escape_char;
        if (0..256).contains(&c) {
            self.print(c);
        }
        self.slow_print(i32::from(s.0));
    }

    /// §64: `print_the_digs(k)`.
    pub fn print_the_digs(&mut self, mut k: u8) {
        while k > 0 {
            k -= 1;
            let d = self.dig[usize::from(k)];
            if d < 10 {
                self.print_char(b'0' + d);
            } else {
                self.print_char(b'A' - 10 + d);
            }
        }
    }

    /// §65: `print_int(n)`, including the overflow-safe `-100000000` branch.
    pub fn print_int(&mut self, mut n: i32) {
        let mut k = 0_u8;
        if n < 0 {
            self.print_char(b'-');
            if n > -100_000_000 {
                n = -n;
            } else {
                let mut m = -1 - n;
                n = m / 10;
                m = (m % 10) + 1;
                k = 1;
                if m < 10 {
                    self.dig[0] = m as u8;
                } else {
                    self.dig[0] = 0;
                    n += 1;
                }
            }
        }
        loop {
            self.dig[usize::from(k)] = (n % 10) as u8;
            n /= 10;
            k += 1;
            if n == 0 {
                break;
            }
        }
        self.print_the_digs(k);
    }

    /// §66: `print_two(n)`.
    pub fn print_two(&mut self, n: i32) {
        let n = n.wrapping_abs() % 100;
        self.print_char(b'0' + (n / 10) as u8);
        self.print_char(b'0' + (n % 10) as u8);
    }

    /// §67: `print_hex(n)` for nonnegative `n`.
    pub fn print_hex(&mut self, mut n: i32) {
        let mut k = 0_u8;
        self.print_char(b'"');
        loop {
            self.dig[usize::from(k)] = (n.rem_euclid(16)) as u8;
            n = n.div_euclid(16);
            k += 1;
            if n == 0 {
                break;
            }
        }
        self.print_the_digs(k);
    }

    /// §69: `print_roman_int(n)`. Uses pooled string 260 (`"m2d5c2l5x2v5i"`).
    pub fn print_roman_int(&mut self, mut n: i32) {
        let mut j = self.str_start[usize::from(StrNum::ROMAN_SPEC.0)];
        let mut v = 1000_i32;
        loop {
            while n >= v {
                self.print_char(self.str_pool[j]);
                n -= v;
            }
            if n <= 0 {
                return;
            }
            let mut k = j + 2;
            let mut u = v / i32::from(self.str_pool[k - 1] - b'0');
            if self.str_pool[k - 1] == b'2' {
                k += 2;
                u /= i32::from(self.str_pool[k - 1] - b'0');
            }
            if n + u >= v {
                self.print_char(self.str_pool[k]);
                n += u;
            } else {
                j += 2;
                v /= i32::from(self.str_pool[j - 1] - b'0');
            }
        }
    }

    /// §70: `print_current_string`.
    pub fn print_current_string(&mut self) {
        let mut j = self.str_start[self.str_ptr];
        while j < self.pool_ptr {
            let ch = self.str_pool[j];
            self.print_char(ch);
            j += 1;
        }
    }

    /// §103: `print_scaled(s)`.
    pub fn print_scaled(&mut self, s: Scaled) {
        let mut s = s.raw();
        if s < 0 {
            self.print_char(b'-');
            s = -s;
        }
        let unity = Scaled::UNITY.raw();
        self.print_int(s / unity);
        self.print_char(b'.');
        s = 10 * (s % unity) + 5;
        let mut delta = 10_i32;
        loop {
            if delta > unity {
                s += 32768 - 50000;
            }
            self.print_char(b'0' + (s / unity) as u8);
            s = 10 * (s % unity);
            delta *= 10;
            if s <= delta {
                break;
            }
        }
    }

    /// §177: `print_glue(d, order, s)`.
    pub fn print_glue(&mut self, d: Scaled, mut order: i32, s: StrNum) {
        self.print_scaled(d);
        if order < i32::from(NORMAL_ORDER) || order > i32::from(FILLL_ORDER) {
            self.print_str(pool_str("foul"));
        } else if order > i32::from(NORMAL_ORDER) {
            self.print_str(pool_str("fil"));
            while order > i32::from(FIL_ORDER) {
                self.print_char(b'l');
                order -= 1;
            }
        } else if s != StrNum::ZERO {
            self.print_str(s);
        }
    }

    /// §178: `print_spec(p, s)`.
    pub fn print_spec(&mut self, memory: &Memory, p: i32, s: StrNum) {
        let stats = memory.stats();
        let mem_min = i32::from(memory.config().mem_bot);
        if p < mem_min || p >= i32::from(stats.lo_mem_max) {
            self.print_char(b'*');
            return;
        }
        let Ok(spec) = memory.glue_spec(MemPtr::from_address(p as u16)) else {
            self.print_char(b'*');
            return;
        };
        self.print_scaled(spec.width);
        if s != StrNum::ZERO {
            self.print_str(s);
        }
        if spec.stretch != Scaled::ZERO {
            self.print_str(pool_str(" plus "));
            self.print_glue(spec.stretch, i32::from(spec.stretch_order), s);
        }
        if spec.shrink != Scaled::ZERO {
            self.print_str(pool_str(" minus "));
            self.print_glue(spec.shrink, i32::from(spec.shrink_order), s);
        }
    }

    /// §71: `term_input`.
    pub fn term_input(&mut self) -> Result<(), Termination> {
        let mut hooks = NoopErrorHooks;
        self.term_input_with_hooks(&mut hooks)
    }

    pub fn term_input_with_hooks(&mut self, hooks: &mut dyn ErrorHooks) -> Result<(), Termination> {
        self.update_terminal();
        let mut term_in = std::mem::take(&mut self.host.term_in);
        let read_res = self.input_ln_with_hooks(&mut term_in, true, hooks);
        self.host.term_in = term_in;
        if !read_res? {
            return Err(self.fatal_error_with_hooks(StrNum::EOF_TERMINAL, hooks));
        }
        self.term_offset = 0;
        self.selector.decr();
        if self.last != self.first {
            for k in self.first..self.last {
                let b = self.buffer[k];
                self.print(i32::from(b));
            }
        }
        self.print_ln();
        self.selector.incr();
        Ok(())
    }

    /// §71: `prompt_input(s)`.
    pub fn prompt_input(
        &mut self,
        s: StrNum,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        self.wake_up_terminal();
        self.print_str(s);
        self.term_input_with_hooks(hooks)
    }

    /// §245: `begin_diagnostic`.
    pub fn begin_diagnostic(&mut self) {
        self.old_setting = self.selector;
        if self.tracing_online <= 0 && self.selector == Selector::TermAndLog {
            self.selector.decr();
            if self.history == History::Spotless {
                self.history = History::WarningIssued;
            }
        }
    }

    /// §245: `end_diagnostic(blank_line)`.
    pub fn end_diagnostic(&mut self, blank_line: bool) {
        self.print_nl(StrNum::EMPTY);
        if blank_line {
            self.print_ln();
        }
        self.selector = self.old_setting;
    }

    /// §73: `print_err(s)`.
    pub fn print_err(&mut self, s: StrNum) {
        if self.interaction == Interaction::ErrorStopMode {
            self.wake_up_terminal();
        }
        self.print_nl(StrNum::BANG);
        self.print_str(s);
    }

    /// §79: Set help lines (`help0` .. `help6`) in source order (`help_line[0]` is printed last).
    pub fn set_help(&mut self, lines: &[StrNum]) {
        assert!(lines.len() <= 6);
        self.help_ptr = lines.len();
        for (i, &s) in lines.iter().rev().enumerate() {
            self.help_line[i] = s;
        }
    }

    /// §81: `jump_out`.
    pub fn jump_out(&mut self) -> Termination {
        Termination::JumpOut
    }

    /// §§82–90: `error`.
    pub fn error(&mut self) -> Result<(), Termination> {
        let mut hooks = NoopErrorHooks;
        self.error_with_hooks(&mut hooks)
    }

    pub fn error_with_hooks(&mut self, hooks: &mut dyn ErrorHooks) -> Result<(), Termination> {
        if self.history < History::ErrorMessageIssued {
            self.history = History::ErrorMessageIssued;
        }
        self.print_char(b'.');
        hooks.show_context(self);
        if self.interaction == Interaction::ErrorStopMode {
            loop {
                if self.interaction != Interaction::ErrorStopMode {
                    return Ok(());
                }
                hooks.clear_for_error_prompt(self);
                self.prompt_input(StrNum::QUESTION_PROMPT, hooks)?;
                if self.last == self.first {
                    return Ok(());
                }
                let mut c = self.buffer[self.first];
                if c >= b'a' {
                    c = c.wrapping_add(b'A').wrapping_sub(b'a');
                }
                match c {
                    b'0'..=b'9' if self.deletions_allowed => {
                        // §88: Delete c - "0" (or two-digit) tokens and goto continue.
                        let s1 = self.cur_tok;
                        let s2 = self.cur_cmd;
                        let s3 = self.cur_chr;
                        let s4 = self.align_state;
                        self.align_state = 1_000_000;
                        self.ok_to_interrupt = false;
                        let mut count = if self.last > self.first + 1
                            && self.buffer[self.first + 1].is_ascii_digit()
                        {
                            usize::from(c - b'0') * 10
                                + usize::from(self.buffer[self.first + 1] - b'0')
                        } else {
                            usize::from(c - b'0')
                        };
                        while count > 0 {
                            hooks.get_token(self)?;
                            count -= 1;
                        }
                        self.cur_tok = s1;
                        self.cur_cmd = s2;
                        self.cur_chr = s3;
                        self.align_state = s4;
                        self.ok_to_interrupt = true;
                        self.set_help(&[StrNum::DELETED_HELP_1, StrNum::DELETED_HELP_0]);
                        hooks.show_context(self);
                        continue;
                    }
                    b'E' if self.base_ptr > 0
                        && self.input_stack[self.base_ptr].name_field.0 >= 256 =>
                    {
                        self.print_nl(StrNum::YOU_WANT_TO_EDIT);
                        self.slow_print(i32::from(self.input_stack[self.base_ptr].name_field.0));
                        self.print_str(StrNum::AT_LINE);
                        self.print_int(self.line);
                        self.interaction = Interaction::ScrollMode;
                        return Err(self.jump_out());
                    }
                    b'H' => {
                        // §89: Print the help information and goto continue.
                        if self.use_err_help {
                            hooks.give_err_help(self);
                            self.use_err_help = false;
                        } else {
                            if self.help_ptr == 0 {
                                self.set_help(&[StrNum::NO_HELP_1, StrNum::NO_HELP_0]);
                            }
                            while self.help_ptr > 0 {
                                self.help_ptr -= 1;
                                let h = self.help_line[self.help_ptr];
                                self.print_str(h);
                                self.print_ln();
                            }
                        }
                        self.set_help(&[
                            StrNum::ALREADY_HELPED_3,
                            StrNum::NO_HELP_0,
                            StrNum::ALREADY_HELPED_1,
                            StrNum::ALREADY_HELPED_0,
                        ]);
                        continue;
                    }
                    b'I' => {
                        // §87: Introduce new material from the terminal and return.
                        hooks.begin_file_reading(self)?;
                        if self.last > self.first + 1 {
                            self.cur_input.loc_field = self.first + 1;
                            self.buffer[self.first] = b' ';
                        } else {
                            self.prompt_input(StrNum::INSERT_PROMPT, hooks)?;
                            self.cur_input.loc_field = self.first;
                        }
                        self.first = self.last;
                        self.cur_input.limit_field = self.last.saturating_sub(1);
                        return Ok(());
                    }
                    b'Q' | b'R' | b'S' => {
                        // §86: Change the interaction level and return.
                        self.error_count = 0;
                        self.interaction =
                            Interaction::from_code(c - b'Q').unwrap_or(Interaction::BatchMode);
                        self.print_str(StrNum::OK_ENTERING);
                        match c {
                            b'Q' => {
                                self.print_esc(StrNum::BATCHMODE);
                                self.selector.decr();
                            }
                            b'R' => self.print_esc(StrNum::NONSTOPMODE),
                            b'S' => self.print_esc(StrNum::SCROLLMODE),
                            _ => unreachable!(),
                        }
                        self.print_str(StrNum::ELLIPSIS);
                        self.print_ln();
                        self.update_terminal();
                        return Ok(());
                    }
                    b'X' => {
                        self.interaction = Interaction::ScrollMode;
                        return Err(self.jump_out());
                    }
                    _ => {}
                }
                // §85: Print the menu of available options.
                self.print_str(StrNum::MENU_PROCEED);
                self.print_nl(StrNum::MENU_RUN);
                self.print_nl(StrNum::MENU_INSERT);
                if self.base_ptr > 0 && self.input_stack[self.base_ptr].name_field.0 >= 256 {
                    self.print_str(StrNum::MENU_EDIT);
                }
                if self.deletions_allowed {
                    self.print_nl(StrNum::MENU_DELETE);
                }
                self.print_nl(StrNum::MENU_HELP_QUIT);
            }
        }
        self.error_count += 1;
        if self.error_count == 100 {
            self.print_nl(StrNum::HUNDRED_ERRORS);
            self.history = History::FatalErrorStop;
            return Err(self.jump_out());
        }
        // §90: Put help message on the transcript file.
        if self.interaction > Interaction::BatchMode {
            self.selector.decr();
        }
        if self.use_err_help {
            self.print_ln();
            hooks.give_err_help(self);
        } else {
            while self.help_ptr > 0 {
                self.help_ptr -= 1;
                let h = self.help_line[self.help_ptr];
                self.print_nl(h);
            }
        }
        self.print_ln();
        if self.interaction > Interaction::BatchMode {
            self.selector.incr();
        }
        self.print_ln();
        Ok(())
    }

    /// §91: `int_error(n)`.
    pub fn int_error(&mut self, n: i32, hooks: &mut dyn ErrorHooks) -> Result<(), Termination> {
        self.print_str(StrNum::SPACE_LPAREN);
        self.print_int(n);
        self.print_char(b')');
        self.error_with_hooks(hooks)
    }

    /// §92: `normalize_selector`.
    pub fn normalize_selector(&mut self, hooks: &mut dyn ErrorHooks) -> Result<(), Termination> {
        if self.log_opened {
            self.selector = Selector::TermAndLog;
        } else {
            self.selector = Selector::TermOnly;
        }
        if self.job_name == StrNum::ZERO {
            self.open_log_file_with_hooks(hooks)?;
        }
        if self.interaction == Interaction::BatchMode {
            self.selector.decr();
        }
        Ok(())
    }

    fn succumb(&mut self, hooks: &mut dyn ErrorHooks) -> Termination {
        if self.interaction == Interaction::ErrorStopMode {
            self.interaction = Interaction::ScrollMode;
        }
        if self.log_opened {
            let _ = self.error_with_hooks(hooks);
        }
        self.history = History::FatalErrorStop;
        self.jump_out()
    }

    /// §93: `fatal_error(s)`.
    pub fn fatal_error(&mut self, s: StrNum) -> Termination {
        let mut hooks = NoopErrorHooks;
        self.fatal_error_with_hooks(s, &mut hooks)
    }

    pub fn fatal_error_with_hooks(&mut self, s: StrNum, hooks: &mut dyn ErrorHooks) -> Termination {
        if let Err(term) = self.normalize_selector(hooks) {
            return term;
        }
        self.print_err(StrNum::EMERGENCY_STOP);
        self.set_help(&[s]);
        self.succumb(hooks)
    }

    /// §94: `overflow(s, n)`.
    pub fn overflow(&mut self, s: StrNum, n: i32) -> Termination {
        let mut hooks = NoopErrorHooks;
        self.overflow_with_hooks(s, n, &mut hooks)
    }

    pub fn overflow_with_hooks(
        &mut self,
        s: StrNum,
        n: i32,
        hooks: &mut dyn ErrorHooks,
    ) -> Termination {
        if let Err(term) = self.normalize_selector(hooks) {
            return term;
        }
        self.print_err(StrNum::CAPACITY_EXCEEDED);
        self.print_str(s);
        self.print_char(b'=');
        self.print_int(n);
        self.print_char(b']');
        self.set_help(&[StrNum::WIZARD_1, StrNum::WIZARD_0]);
        self.succumb(hooks)
    }

    /// §95: `confusion(s)`.
    pub fn confusion(&mut self, s: StrNum) -> Termination {
        let mut hooks = NoopErrorHooks;
        self.confusion_with_hooks(s, &mut hooks)
    }

    pub fn confusion_with_hooks(&mut self, s: StrNum, hooks: &mut dyn ErrorHooks) -> Termination {
        if let Err(term) = self.normalize_selector(hooks) {
            return term;
        }
        if self.history < History::ErrorMessageIssued {
            self.print_err(StrNum::THIS_CANT_HAPPEN);
            self.print_str(s);
            self.print_char(b')');
            self.set_help(&[StrNum::IM_BROKEN]);
        } else {
            self.print_err(StrNum::CANT_GO_ON);
            self.set_help(&[StrNum::FAUX_PAS_1, StrNum::FAUX_PAS_0]);
        }
        self.succumb(hooks)
    }

    /// §96: `check_interrupt`.
    pub fn check_interrupt(&mut self, hooks: &mut dyn ErrorHooks) -> Result<(), Termination> {
        if self.interrupt != 0 {
            self.pause_for_instructions(hooks)?;
        }
        Ok(())
    }

    /// §98: `pause_for_instructions`.
    pub fn pause_for_instructions(
        &mut self,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        if self.ok_to_interrupt {
            self.interaction = Interaction::ErrorStopMode;
            if self.selector == Selector::LogOnly || self.selector == Selector::NoPrint {
                self.selector.incr();
            }
            self.print_err(StrNum::INTERRUPTION);
            self.set_help(&[StrNum::YOU_RANG_2, StrNum::YOU_RANG_1, StrNum::YOU_RANG_0]);
            self.deletions_allowed = false;
            self.error_with_hooks(hooks)?;
            self.deletions_allowed = true;
            self.interrupt = 0;
        }
        Ok(())
    }

    /// §515: `begin_name`.
    pub fn begin_name(&mut self) {
        self.area_delimiter = 0;
        self.ext_delimiter = 0;
    }

    /// §516: `more_name(c)`.
    pub fn more_name(&mut self, c: u8) -> Result<bool, Termination> {
        let mut hooks = NoopErrorHooks;
        self.more_name_with_hooks(c, &mut hooks)
    }

    pub fn more_name_with_hooks(
        &mut self,
        c: u8,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<bool, Termination> {
        if c == b' ' {
            Ok(false)
        } else {
            self.str_room_with_hooks(1, hooks)?;
            self.append_char(c);
            if c == b'>' || c == b':' {
                self.area_delimiter = self.cur_length();
                self.ext_delimiter = 0;
            } else if c == b'.' && self.ext_delimiter == 0 {
                self.ext_delimiter = self.cur_length();
            }
            Ok(true)
        }
    }

    /// §517: `end_name`.
    pub fn end_name(&mut self) -> Result<(), Termination> {
        let mut hooks = NoopErrorHooks;
        self.end_name_with_hooks(&mut hooks)
    }

    pub fn end_name_with_hooks(&mut self, hooks: &mut dyn ErrorHooks) -> Result<(), Termination> {
        if self.str_ptr + 3 > self.config.max_strings {
            let n = (self.config.max_strings - self.init_str_ptr) as i32;
            return Err(self.overflow_with_hooks(StrNum::NUMBER_OF_STRINGS, n, hooks));
        }
        if self.area_delimiter == 0 {
            self.cur_area = StrNum::EMPTY;
        } else {
            self.cur_area = StrNum(self.str_ptr as u16);
            self.str_start[self.str_ptr + 1] = self.str_start[self.str_ptr] + self.area_delimiter;
            self.str_ptr += 1;
        }
        if self.ext_delimiter == 0 {
            self.cur_ext = StrNum::EMPTY;
            self.cur_name = self.make_string_with_hooks(hooks)?;
        } else {
            self.cur_name = StrNum(self.str_ptr as u16);
            self.str_start[self.str_ptr + 1] =
                self.str_start[self.str_ptr] + self.ext_delimiter - self.area_delimiter - 1;
            self.str_ptr += 1;
            self.cur_ext = self.make_string_with_hooks(hooks)?;
        }
        Ok(())
    }

    /// §518: `print_file_name(n, a, e)`.
    pub fn print_file_name(&mut self, n: StrNum, a: StrNum, e: StrNum) {
        self.slow_print(i32::from(a.0));
        self.slow_print(i32::from(n.0));
        self.slow_print(i32::from(e.0));
    }

    /// §519: `pack_file_name(n, a, e)`.
    pub fn pack_file_name(&mut self, n: StrNum, a: StrNum, e: StrNum) {
        let mut k = 0_usize;
        for s in [a, n, e] {
            let idx = usize::from(s.0);
            for j in self.str_start[idx]..self.str_start[idx + 1] {
                let c = self.str_pool[j];
                k += 1;
                if k <= self.config.file_name_size {
                    self.name_of_file[k - 1] = self.chars.xchr(c);
                }
            }
        }
        self.name_length = k.min(self.config.file_name_size);
        for slot in &mut self.name_of_file[self.name_length..self.config.file_name_size] {
            *slot = b' ';
        }
    }

    /// §529: `pack_cur_name`.
    pub fn pack_cur_name(&mut self) {
        self.pack_file_name(self.cur_name, self.cur_area, self.cur_ext);
    }

    /// §523: `pack_buffered_name(n, a, b)`.
    pub fn pack_buffered_name(&mut self, n: usize, a: isize, mut b: isize) {
        let max_len = self.config.file_name_size as isize;
        let n_isize = n as isize;
        let ext_len = FORMAT_EXT_LENGTH as isize;
        if n_isize + b - a + 1 + ext_len > max_len {
            b = a + max_len - n_isize - 1 - ext_len;
        }
        let mut k = 0_usize;
        for j in 0..n.min(FORMAT_DEFAULT_LENGTH) {
            let c = self.chars.xord(self.tex_format_default[j]);
            k += 1;
            if k <= self.config.file_name_size {
                self.name_of_file[k - 1] = self.chars.xchr(c);
            }
        }
        if a <= b {
            for j in a..=b {
                if let Ok(idx) = usize::try_from(j) {
                    let c = self.buffer[idx];
                    k += 1;
                    if k <= self.config.file_name_size {
                        self.name_of_file[k - 1] = self.chars.xchr(c);
                    }
                }
            }
        }
        for j in (FORMAT_DEFAULT_LENGTH - FORMAT_EXT_LENGTH)..FORMAT_DEFAULT_LENGTH {
            let c = self.chars.xord(self.tex_format_default[j]);
            k += 1;
            if k <= self.config.file_name_size {
                self.name_of_file[k - 1] = self.chars.xchr(c);
            }
        }
        self.name_length = k.min(self.config.file_name_size);
        for slot in &mut self.name_of_file[self.name_length..self.config.file_name_size] {
            *slot = b' ';
        }
    }

    /// Current `name_of_file[0..name_length]` as a byte slice.
    pub fn packed_file_name_bytes(&self) -> &[u8] {
        &self.name_of_file[..self.name_length]
    }

    /// §524: `open_fmt_file`.
    pub fn open_fmt_file(&mut self) -> Option<ByteFileIn> {
        let mut j = self.cur_input.loc_field;
        if self.buffer[self.cur_input.loc_field] == b'&' {
            self.cur_input.loc_field += 1;
            j = self.cur_input.loc_field;
            self.buffer[self.last] = b' ';
            while self.buffer[j] != b' ' {
                j += 1;
            }
            self.pack_buffered_name(0, self.cur_input.loc_field as isize, (j as isize) - 1);
            let name = String::from_utf8_lossy(&self.name_of_file).into_owned();
            if let Some(f) = self.host.b_open_in(&name) {
                self.cur_input.loc_field = j;
                return Some(f);
            }
            self.pack_buffered_name(
                FORMAT_AREA_LENGTH,
                self.cur_input.loc_field as isize,
                (j as isize) - 1,
            );
            let area_name = String::from_utf8_lossy(&self.name_of_file).into_owned();
            if let Some(f) = self.host.b_open_in(&area_name) {
                self.cur_input.loc_field = j;
                return Some(f);
            }
            self.wake_up_terminal();
            self.wterm_ln(b"Sorry, I can't find that format; will try PLAIN.");
            self.update_terminal();
        }
        self.pack_buffered_name(FORMAT_DEFAULT_LENGTH - FORMAT_EXT_LENGTH, 1, 0);
        let plain_name = String::from_utf8_lossy(&self.name_of_file).into_owned();
        if let Some(f) = self.host.b_open_in(&plain_name) {
            self.cur_input.loc_field = j;
            Some(f)
        } else {
            self.wake_up_terminal();
            self.wterm_ln(b"I can't find the PLAIN format file!");
            None
        }
    }

    /// §525: `make_name_string` (`a_make_name_string`, `b_make_name_string`, `w_make_name_string`).
    pub fn make_name_string(&mut self) -> Result<StrNum, Termination> {
        if self.pool_ptr + self.name_length > self.config.pool_size
            || self.str_ptr == self.config.max_strings
            || self.cur_length() > 0
        {
            Ok(StrNum::from_ascii(b'?'))
        } else {
            for k in 0..self.name_length {
                let ord = self.chars.xord(self.name_of_file[k]);
                self.append_char(ord);
            }
            self.make_string()
        }
    }

    /// §529: `pack_job_name(s)`.
    pub fn pack_job_name(&mut self, s: StrNum) {
        self.cur_area = StrNum::EMPTY;
        self.cur_ext = s;
        self.cur_name = self.job_name;
        self.pack_cur_name();
    }

    /// §§530–531: `prompt_file_name(s, e)`.
    pub fn prompt_file_name(
        &mut self,
        s: StrNum,
        e: StrNum,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        if self.interaction == Interaction::ScrollMode {
            self.wake_up_terminal();
        }
        if s == StrNum::INPUT_FILE_NAME {
            self.print_err(StrNum::CANT_FIND_FILE);
        } else {
            self.print_err(StrNum::CANT_WRITE_FILE);
        }
        self.print_file_name(self.cur_name, self.cur_area, self.cur_ext);
        self.print_str(StrNum::QUOTE_DOT);
        if e == StrNum::DOT_TEX {
            hooks.show_context(self);
        }
        self.print_nl(StrNum::PLEASE_TYPE_ANOTHER);
        self.print_str(s);
        if self.interaction < Interaction::ScrollMode {
            return Err(self.fatal_error_with_hooks(StrNum::ABORTED_FILE_ERROR, hooks));
        }
        self.clear_terminal();
        self.prompt_input(StrNum::COLON_PROMPT, hooks)?;
        // §531: Scan file name in the buffer.
        self.begin_name();
        let mut k = self.first;
        while self.buffer[k] == b' ' && k < self.last {
            k += 1;
        }
        while k < self.last {
            let b = self.buffer[k];
            if !self.more_name_with_hooks(b, hooks)? {
                break;
            }
            k += 1;
        }
        self.end_name_with_hooks(hooks)?;
        if self.cur_ext == StrNum::EMPTY {
            self.cur_ext = e;
        }
        self.pack_cur_name();
        Ok(())
    }

    /// §§534–536: `open_log_file`.
    pub fn open_log_file(&mut self) -> Result<(), Termination> {
        let mut hooks = NoopErrorHooks;
        self.open_log_file_with_hooks(&mut hooks)
    }

    pub fn open_log_file_with_hooks(
        &mut self,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        let old_setting = self.selector;
        if self.job_name == StrNum::ZERO {
            self.job_name = StrNum::TEXPUT;
        }
        self.pack_job_name(StrNum::DOT_LOG);
        loop {
            let candidate = String::from_utf8_lossy(&self.name_of_file).into_owned();
            if self.host.can_open_out(&candidate) {
                break;
            }
            // §535: Try to get a different log file name.
            self.selector = Selector::TermOnly;
            self.prompt_file_name(StrNum::TRANSCRIPT_FILE_NAME, StrNum::DOT_LOG, hooks)?;
        }
        self.log_name = self.make_name_string()?;
        self.selector = Selector::LogOnly;
        self.log_opened = true;

        // §536: Print the banner line, including the date and time.
        self.wlog(BANNER.as_bytes());
        self.slow_print(i32::from(self.format_ident.0));
        self.print_str(StrNum::TWO_SPACES);
        self.print_int(self.sys_day);
        self.print_char(b' ');
        let months = b"JANFEBMARAPRMAYJUNJULAUGSEPOCTNOVDEC";
        if (1..=12).contains(&self.sys_month) {
            let start = (3 * (self.sys_month as usize)) - 3;
            self.wlog(&months[start..start + 3]);
        }
        self.print_char(b' ');
        self.print_int(self.sys_year);
        self.print_char(b' ');
        self.print_two(self.sys_time / 60);
        self.print_char(b':');
        self.print_two(self.sys_time % 60);

        self.input_stack[self.input_ptr] = self.cur_input;
        self.print_nl(StrNum::DOUBLE_STAR);
        let mut l = self.input_stack[0].limit_field;
        if l > 0 && i32::from(self.buffer[l]) == self.end_line_char {
            l -= 1;
        }
        for k in 1..=l {
            let b = self.buffer[k];
            self.print(i32::from(b));
        }
        self.print_ln();
        self.selector = Selector::from_code(old_setting.code() + 2).unwrap_or(Selector::TermAndLog);
        Ok(())
    }

    /// §532: `ensure_dvi_open`.
    pub fn ensure_dvi_open(&mut self, hooks: &mut dyn ErrorHooks) -> Result<(), Termination> {
        if self.output_file_name == StrNum::ZERO {
            if self.job_name == StrNum::ZERO {
                self.open_log_file_with_hooks(hooks)?;
            }
            self.pack_job_name(StrNum::DOT_DVI);
            loop {
                let candidate = String::from_utf8_lossy(&self.name_of_file).into_owned();
                if self.host.can_open_out(&candidate) {
                    break;
                }
                self.prompt_file_name(StrNum::FILE_NAME_FOR_OUTPUT, StrNum::DOT_DVI, hooks)?;
            }
            self.output_file_name = self.make_name_string()?;
        }
        Ok(())
    }

    /// §321: `push_input`.
    pub fn push_input(&mut self, hooks: &mut dyn ErrorHooks) -> Result<(), Termination> {
        if self.input_ptr > self.max_in_stack {
            self.max_in_stack = self.input_ptr;
            if self.input_ptr == self.config.stack_size {
                return Err(self.overflow_with_hooks(
                    pool_str("input stack size"),
                    self.config.stack_size as i32,
                    hooks,
                ));
            }
        }
        self.input_stack[self.input_ptr] = self.cur_input;
        self.input_ptr += 1;
        Ok(())
    }

    /// §322: `pop_input`.
    pub fn pop_input(&mut self) {
        self.input_ptr -= 1;
        self.cur_input = self.input_stack[self.input_ptr];
    }

    /// §328: `begin_file_reading`.
    pub fn begin_file_reading_with_hooks(
        &mut self,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        if self.in_open == self.config.max_in_open {
            return Err(self.overflow_with_hooks(
                pool_str("text input levels"),
                self.config.max_in_open as i32,
                hooks,
            ));
        }
        if self.first == self.config.buf_size {
            return Err(self.overflow_with_hooks(
                StrNum::BUFFER_SIZE,
                self.config.buf_size as i32,
                hooks,
            ));
        }
        self.in_open += 1;
        self.push_input(hooks)?;
        self.cur_input.index_field = self.in_open as u8;
        self.line_stack[self.in_open] = self.line;
        self.cur_input.start_field = self.first;
        self.cur_input.state_field = 1; // mid_line
        self.cur_input.name_field = StrNum::ZERO;
        Ok(())
    }

    /// Default M3 input-stack push for `begin_file_reading` (§328).
    pub fn begin_file_reading_default(&mut self) {
        let mut hooks = NoopErrorHooks;
        let _ = self.begin_file_reading_with_hooks(&mut hooks);
    }

    /// Default M3 input-stack pop for `end_file_reading` (§329).
    pub fn end_file_reading_default(&mut self) {
        self.first = self.cur_input.start_field;
        self.line = self.line_stack[usize::from(self.cur_input.index_field)];
        if self.cur_input.name_field.0 > 17 {
            self.input_file[usize::from(self.cur_input.index_field)].close();
        }
        self.pop_input();
        self.in_open -= 1;
    }

    /// §330: `clear_for_error_prompt`.
    pub fn clear_for_error_prompt_default(&mut self) {
        while self.cur_input.state_field != 0
            && self.cur_input.name_field == StrNum::ZERO
            && self.input_ptr > 0
            && self.cur_input.loc_field > self.cur_input.limit_field
        {
            self.end_file_reading_default();
        }
        self.print_ln();
        self.clear_terminal();
    }

    /// §363: `firm_up_the_line`.
    pub fn firm_up_the_line(&mut self, hooks: &mut dyn ErrorHooks) -> Result<(), Termination> {
        self.cur_input.limit_field = self.last;
        if self.pausing > 0 && self.interaction > Interaction::NonstopMode {
            self.wake_up_terminal();
            self.print_ln();
            let start = self.cur_input.start_field;
            let limit = self.cur_input.limit_field;
            if start < limit {
                for k in start..limit {
                    let b = self.buffer[k];
                    self.print(i32::from(b));
                }
            }
            self.first = limit;
            self.prompt_input(pool_str("=>"), hooks)?;
            if self.last > self.first {
                for k in self.first..self.last {
                    self.buffer[k + start - self.first] = self.buffer[k];
                }
                self.cur_input.limit_field = start + self.last - self.first;
            }
        }
        Ok(())
    }

    /// §§537–538: File lookup, log opening, `(name` printing, pool conservation, and first-line reading after `cur_name`/`cur_area`/`cur_ext` are set.
    pub fn start_input_after_scan(
        &mut self,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        if self.cur_ext == StrNum::EMPTY {
            self.cur_ext = StrNum::DOT_TEX;
        }
        self.pack_cur_name();
        loop {
            hooks.begin_file_reading(self)?;
            let candidate = String::from_utf8_lossy(&self.name_of_file).into_owned();
            if let Some(f) = self.host.a_open_in(&candidate) {
                self.input_file[usize::from(self.cur_input.index_field)] = f;
                break;
            }
            if self.cur_area == StrNum::EMPTY {
                self.pack_file_name(self.cur_name, StrNum::TEX_AREA_STR, self.cur_ext);
                let fallback = String::from_utf8_lossy(&self.name_of_file).into_owned();
                if let Some(f) = self.host.a_open_in(&fallback) {
                    self.input_file[usize::from(self.cur_input.index_field)] = f;
                    break;
                }
            }
            hooks.end_file_reading(self);
            self.prompt_file_name(StrNum::INPUT_FILE_NAME, StrNum::DOT_TEX, hooks)?;
        }
        self.cur_input.name_field = self.make_name_string()?;
        if self.job_name == StrNum::ZERO {
            self.job_name = self.cur_name;
            self.open_log_file_with_hooks(hooks)?;
        }
        let name_len = self.length(self.cur_input.name_field);
        if self.term_offset + name_len > self.config.max_print_line - 2 {
            self.print_ln();
        } else if self.term_offset > 0 || self.file_offset > 0 {
            self.print_char(b' ');
        }
        self.print_char(b'(');
        self.open_parens += 1;
        self.slow_print(i32::from(self.cur_input.name_field.0));
        self.update_terminal();
        self.cur_input.state_field = 33; // new_line
        if usize::from(self.cur_input.name_field.0) + 1 == self.str_ptr {
            self.flush_string();
            self.cur_input.name_field = self.cur_name;
        }
        // §538: Read the first line of the new file.
        self.line = 1;
        let idx = usize::from(self.cur_input.index_field);
        let mut file = std::mem::take(&mut self.input_file[idx]);
        let res = self.input_ln_with_hooks(&mut file, false, hooks);
        self.input_file[idx] = file;
        let _ = res?;
        self.firm_up_the_line(hooks)?;
        if self.end_line_char < 0 || self.end_line_char > 255 {
            self.cur_input.limit_field = self.cur_input.limit_field.saturating_sub(1);
        } else {
            self.buffer[self.cur_input.limit_field] = self.end_line_char as u8;
        }
        self.first = self.cur_input.limit_field + 1;
        self.cur_input.loc_field = self.cur_input.start_field;
        Ok(())
    }
}
