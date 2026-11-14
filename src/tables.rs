//! Knuth, tex.web §§207–299: Command codes, semantic nest, `eqtb`, hash table, save stack, and token lists.
//! Detailed contracts: docs/semantics.md.

use crate::arithmetic::Scaled;
use crate::io::{
    CARRIAGE_RETURN, ErrorHooks, INVALID_CODE, IoState, NULL_CODE, StrNum, Termination, pool_str,
};
use crate::memory::{MemPtr, Memory, MemoryError};

/// §§207–210: Command codes (`0..=120`).
pub mod cmd {
    pub const ESCAPE: u8 = 0;
    pub const RELAX: u8 = 0;
    pub const LEFT_BRACE: u8 = 1;
    pub const RIGHT_BRACE: u8 = 2;
    pub const MATH_SHIFT: u8 = 3;
    pub const TAB_MARK: u8 = 4;
    pub const CAR_RET: u8 = 5;
    pub const OUT_PARAM: u8 = 5;
    pub const MAC_PARAM: u8 = 6;
    pub const SUP_MARK: u8 = 7;
    pub const SUB_MARK: u8 = 8;
    pub const IGNORE: u8 = 9;
    pub const ENDV: u8 = 9;
    pub const SPACER: u8 = 10;
    pub const LETTER: u8 = 11;
    pub const OTHER_CHAR: u8 = 12;
    pub const ACTIVE_CHAR: u8 = 13;
    pub const PAR_END: u8 = 13;
    pub const MATCH: u8 = 13;
    pub const COMMENT: u8 = 14;
    pub const END_MATCH: u8 = 14;
    pub const STOP: u8 = 14;
    pub const INVALID_CHAR: u8 = 15;
    pub const DELIM_NUM: u8 = 15;
    pub const MAX_CHAR_CODE: u8 = 15;

    pub const CHAR_NUM: u8 = 16;
    pub const MATH_CHAR_NUM: u8 = 17;
    pub const MARK: u8 = 18;
    pub const XRAY: u8 = 19;
    pub const MAKE_BOX: u8 = 20;
    pub const HMOVE: u8 = 21;
    pub const VMOVE: u8 = 22;
    pub const UN_HBOX: u8 = 23;
    pub const UN_VBOX: u8 = 24;
    pub const REMOVE_ITEM: u8 = 25;
    pub const HSKIP: u8 = 26;
    pub const VSKIP: u8 = 27;
    pub const MSKIP: u8 = 28;
    pub const KERN: u8 = 29;
    pub const MKERN: u8 = 30;
    pub const LEADER_SHIP: u8 = 31;
    pub const HALIGN: u8 = 32;
    pub const VALIGN: u8 = 33;
    pub const NO_ALIGN: u8 = 34;
    pub const VRULE: u8 = 35;
    pub const HRULE: u8 = 36;
    pub const INSERT: u8 = 37;
    pub const VADJUST: u8 = 38;
    pub const IGNORE_SPACES: u8 = 39;
    pub const AFTER_ASSIGNMENT: u8 = 40;
    pub const AFTER_GROUP: u8 = 41;
    pub const BREAK_PENALTY: u8 = 42;
    pub const START_PAR: u8 = 43;
    pub const ITAL_CORR: u8 = 44;
    pub const ACCENT: u8 = 45;
    pub const MATH_ACCENT: u8 = 46;
    pub const DISCRETIONARY: u8 = 47;
    pub const EQ_NO: u8 = 48;
    pub const LEFT_RIGHT: u8 = 49;
    pub const MATH_COMP: u8 = 50;
    pub const LIMIT_SWITCH: u8 = 51;
    pub const ABOVE: u8 = 52;
    pub const MATH_STYLE: u8 = 53;
    pub const MATH_CHOICE: u8 = 54;
    pub const NON_SCRIPT: u8 = 55;
    pub const VCENTER: u8 = 56;
    pub const CASE_SHIFT: u8 = 57;
    pub const MESSAGE: u8 = 58;
    pub const EXTENSION: u8 = 59;
    pub const IN_STREAM: u8 = 60;
    pub const BEGIN_GROUP: u8 = 61;
    pub const END_GROUP: u8 = 62;
    pub const OMIT: u8 = 63;
    pub const EX_SPACE: u8 = 64;
    pub const NO_BOUNDARY: u8 = 65;
    pub const RADICAL: u8 = 66;
    pub const END_CS_NAME: u8 = 67;
    pub const MIN_INTERNAL: u8 = 68;
    pub const CHAR_GIVEN: u8 = 68;
    pub const MATH_GIVEN: u8 = 69;
    pub const LAST_ITEM: u8 = 70;
    pub const MAX_NON_PREFIXED_COMMAND: u8 = 70;

    pub const TOKS_REGISTER: u8 = 71;
    pub const ASSIGN_TOKS: u8 = 72;
    pub const ASSIGN_INT: u8 = 73;
    pub const ASSIGN_DIMEN: u8 = 74;
    pub const ASSIGN_GLUE: u8 = 75;
    pub const ASSIGN_MU_GLUE: u8 = 76;
    pub const ASSIGN_FONT_DIMEN: u8 = 77;
    pub const ASSIGN_FONT_INT: u8 = 78;
    pub const SET_AUX: u8 = 79;
    pub const SET_PREV_GRAF: u8 = 80;
    pub const SET_PAGE_DIMEN: u8 = 81;
    pub const SET_PAGE_INT: u8 = 82;
    pub const SET_BOX_DIMEN: u8 = 83;
    pub const SET_SHAPE: u8 = 84;
    pub const DEF_CODE: u8 = 85;
    pub const DEF_FAMILY: u8 = 86;
    pub const SET_FONT: u8 = 87;
    pub const DEF_FONT: u8 = 88;
    pub const REGISTER: u8 = 89;
    pub const MAX_INTERNAL: u8 = 89;
    pub const ADVANCE: u8 = 90;
    pub const MULTIPLY: u8 = 91;
    pub const DIVIDE: u8 = 92;
    pub const PREFIX: u8 = 93;
    pub const LET: u8 = 94;
    pub const SHORTHAND_DEF: u8 = 95;
    pub const READ_TO_CS: u8 = 96;
    pub const DEF: u8 = 97;
    pub const SET_BOX: u8 = 98;
    pub const HYPH_DATA: u8 = 99;
    pub const SET_INTERACTION: u8 = 100;
    pub const MAX_COMMAND: u8 = 100;

    pub const UNDEFINED_CS: u8 = 101;
    pub const EXPAND_AFTER: u8 = 102;
    pub const NO_EXPAND: u8 = 103;
    pub const INPUT: u8 = 104;
    pub const IF_TEST: u8 = 105;
    pub const FI_OR_ELSE: u8 = 106;
    pub const CS_NAME: u8 = 107;
    pub const CONVERT: u8 = 108;
    pub const THE: u8 = 109;
    pub const TOP_BOT_MARK: u8 = 110;
    pub const CALL: u8 = 111;
    pub const LONG_CALL: u8 = 112;
    pub const OUTER_CALL: u8 = 113;
    pub const LONG_OUTER_CALL: u8 = 114;
    pub const END_TEMPLATE: u8 = 115;
    pub const DONT_EXPAND: u8 = 116;
    pub const GLUE_REF: u8 = 117;
    pub const SHAPE_REF: u8 = 118;
    pub const BOX_REF: u8 = 119;
    pub const DATA: u8 = 120;
}

/// §211: Semantic modes.
pub const VMODE: i32 = 1;
pub const HMODE: i32 = VMODE + (cmd::MAX_COMMAND as i32) + 1; // 102
pub const MMODE: i32 = HMODE + (cmd::MAX_COMMAND as i32) + 1; // 203
/// §212: `ignore_depth == -65536000` (`-1000pt`).
pub const IGNORE_DEPTH: i32 = -65_536_000;

/// §221: Grouping level constants.
pub const LEVEL_ZERO: u8 = 0;
pub const LEVEL_ONE: u8 = 1;

/// §232: Font and math code constants.
pub const FONT_BASE: u16 = 0;
pub const NULL_FONT: u16 = FONT_BASE;
pub const VAR_CODE: u16 = 0o70000; // 28672

/// §224: Glue parameter codes (`0..=17`).
pub const LINE_SKIP_CODE: usize = 0;
pub const BASELINE_SKIP_CODE: usize = 1;
pub const PAR_SKIP_CODE: usize = 2;
pub const ABOVE_DISPLAY_SKIP_CODE: usize = 3;
pub const BELOW_DISPLAY_SKIP_CODE: usize = 4;
pub const ABOVE_DISPLAY_SHORT_SKIP_CODE: usize = 5;
pub const BELOW_DISPLAY_SHORT_SKIP_CODE: usize = 6;
pub const LEFT_SKIP_CODE: usize = 7;
pub const RIGHT_SKIP_CODE: usize = 8;
pub const TOP_SKIP_CODE: usize = 9;
pub const SPLIT_TOP_SKIP_CODE: usize = 10;
pub const TAB_SKIP_CODE: usize = 11;
pub const SPACE_SKIP_CODE: usize = 12;
pub const XSPACE_SKIP_CODE: usize = 13;
pub const PAR_FILL_SKIP_CODE: usize = 14;
pub const THIN_MU_SKIP_CODE: usize = 15;
pub const MED_MU_SKIP_CODE: usize = 16;
pub const THICK_MU_SKIP_CODE: usize = 17;
pub const GLUE_PARS: usize = 18;

/// §236: Integer parameter codes (`0..=54`).
pub const PRETOLERANCE_CODE: usize = 0;
pub const TOLERANCE_CODE: usize = 1;
pub const LINE_PENALTY_CODE: usize = 2;
pub const HYPHEN_PENALTY_CODE: usize = 3;
pub const EX_HYPHEN_PENALTY_CODE: usize = 4;
pub const CLUB_PENALTY_CODE: usize = 5;
pub const WIDOW_PENALTY_CODE: usize = 6;
pub const DISPLAY_WIDOW_PENALTY_CODE: usize = 7;
pub const BROKEN_PENALTY_CODE: usize = 8;
pub const BIN_OP_PENALTY_CODE: usize = 9;
pub const REL_PENALTY_CODE: usize = 10;
pub const PRE_DISPLAY_PENALTY_CODE: usize = 11;
pub const POST_DISPLAY_PENALTY_CODE: usize = 12;
pub const INTER_LINE_PENALTY_CODE: usize = 13;
pub const DOUBLE_HYPHEN_DEMERITS_CODE: usize = 14;
pub const FINAL_HYPHEN_DEMERITS_CODE: usize = 15;
pub const ADJ_DEMERITS_CODE: usize = 16;
pub const MAG_CODE: usize = 17;
pub const DELIMITER_FACTOR_CODE: usize = 18;
pub const LOOSENESS_CODE: usize = 19;
pub const TIME_CODE: usize = 20;
pub const DAY_CODE: usize = 21;
pub const MONTH_CODE: usize = 22;
pub const YEAR_CODE: usize = 23;
pub const SHOW_BOX_BREADTH_CODE: usize = 24;
pub const SHOW_BOX_DEPTH_CODE: usize = 25;
pub const HBADNESS_CODE: usize = 26;
pub const VBADNESS_CODE: usize = 27;
pub const PAUSING_CODE: usize = 28;
pub const TRACING_ONLINE_CODE: usize = 29;
pub const TRACING_MACROS_CODE: usize = 30;
pub const TRACING_STATS_CODE: usize = 31;
pub const TRACING_PARAGRAPHS_CODE: usize = 32;
pub const TRACING_PAGES_CODE: usize = 33;
pub const TRACING_OUTPUT_CODE: usize = 34;
pub const TRACING_LOST_CHARS_CODE: usize = 35;
pub const TRACING_COMMANDS_CODE: usize = 36;
pub const TRACING_RESTORES_CODE: usize = 37;
pub const UC_HYPH_CODE: usize = 38;
pub const OUTPUT_PENALTY_CODE: usize = 39;
pub const MAX_DEAD_CYCLES_CODE: usize = 40;
pub const HANG_AFTER_CODE: usize = 41;
pub const FLOATING_PENALTY_CODE: usize = 42;
pub const GLOBAL_DEFS_CODE: usize = 43;
pub const CUR_FAM_CODE: usize = 44;
pub const ESCAPE_CHAR_CODE: usize = 45;
pub const DEFAULT_HYPHEN_CHAR_CODE: usize = 46;
pub const DEFAULT_SKEW_CHAR_CODE: usize = 47;
pub const END_LINE_CHAR_CODE: usize = 48;
pub const NEW_LINE_CHAR_CODE: usize = 49;
pub const LANGUAGE_CODE: usize = 50;
pub const LEFT_HYPHEN_MIN_CODE: usize = 51;
pub const RIGHT_HYPHEN_MIN_CODE: usize = 52;
pub const HOLDING_INSERTS_CODE: usize = 53;
pub const ERROR_CONTEXT_LINES_CODE: usize = 54;
pub const INT_PARS: usize = 55;

/// §247: Dimension parameter codes (`0..=20`).
pub const PAR_INDENT_CODE: usize = 0;
pub const MATH_SURROUND_CODE: usize = 1;
pub const LINE_SKIP_LIMIT_CODE: usize = 2;
pub const HSIZE_CODE: usize = 3;
pub const VSIZE_CODE: usize = 4;
pub const MAX_DEPTH_CODE: usize = 5;
pub const SPLIT_MAX_DEPTH_CODE: usize = 6;
pub const BOX_MAX_DEPTH_CODE: usize = 7;
pub const HFUZZ_CODE: usize = 8;
pub const VFUZZ_CODE: usize = 9;
pub const DELIMITER_SHORTFALL_CODE: usize = 10;
pub const NULL_DELIMITER_SPACE_CODE: usize = 11;
pub const SCRIPT_SPACE_CODE: usize = 12;
pub const PRE_DISPLAY_SIZE_CODE: usize = 13;
pub const DISPLAY_WIDTH_CODE: usize = 14;
pub const DISPLAY_INDENT_CODE: usize = 15;
pub const OVERFULL_RULE_CODE: usize = 16;
pub const HANG_INDENT_CODE: usize = 17;
pub const H_OFFSET_CODE: usize = 18;
pub const V_OFFSET_CODE: usize = 19;
pub const EMERGENCY_STRETCH_CODE: usize = 20;
pub const DIMEN_PARS: usize = 21;

/// §§382, 410, 416, 468, 487, 489, 780, 1058, 1071, 1222, 1291, 1341: Modifier codes for primitives.
pub const TOP_MARK_CODE: u16 = 0;
pub const FIRST_MARK_CODE: u16 = 1;
pub const BOT_MARK_CODE: u16 = 2;
pub const SPLIT_FIRST_MARK_CODE: u16 = 3;
pub const SPLIT_BOT_MARK_CODE: u16 = 4;

pub const INT_VAL: u16 = 0;
pub const DIMEN_VAL: u16 = 1;
pub const GLUE_VAL: u16 = 2;
pub const MU_VAL: u16 = 3;
pub const IDENT_VAL: u16 = 4;
pub const TOK_VAL: u16 = 5;
pub const INPUT_LINE_NO_CODE: u16 = GLUE_VAL + 1;
pub const BADNESS_CODE: u16 = GLUE_VAL + 2;

pub const WIDTH_OFFSET: u16 = 1;
pub const DEPTH_OFFSET: u16 = 2;
pub const HEIGHT_OFFSET: u16 = 3;

pub const NUMBER_CODE: u16 = 0;
pub const ROMAN_NUMERAL_CODE: u16 = 1;
pub const STRING_CODE: u16 = 2;
pub const MEANING_CODE: u16 = 3;
pub const FONT_NAME_CODE: u16 = 4;
pub const JOB_NAME_CODE: u16 = 5;

pub const IF_CHAR_CODE: u16 = 0;
pub const IF_CAT_CODE: u16 = 1;
pub const IF_INT_CODE: u16 = 2;
pub const IF_DIM_CODE: u16 = 3;
pub const IF_ODD_CODE: u16 = 4;
pub const IF_VMODE_CODE: u16 = 5;
pub const IF_HMODE_CODE: u16 = 6;
pub const IF_MMODE_CODE: u16 = 7;
pub const IF_INNER_CODE: u16 = 8;
pub const IF_VOID_CODE: u16 = 9;
pub const IF_HBOX_CODE: u16 = 10;
pub const IF_VBOX_CODE: u16 = 11;
pub const IFX_CODE: u16 = 12;
pub const IF_EOF_CODE: u16 = 13;
pub const IF_TRUE_CODE: u16 = 14;
pub const IF_FALSE_CODE: u16 = 15;
pub const IF_CASE_CODE: u16 = 16;

pub const IF_CODE: u8 = 1;
pub const FI_CODE: u16 = 2;
pub const ELSE_CODE: u16 = 3;
pub const OR_CODE: u16 = 4;

pub const SPAN_CODE: u16 = 256;
pub const CR_CODE: u16 = 257;
pub const CR_CR_CODE: u16 = 258;

pub const FIL_CODE: u16 = 0;
pub const FILL_CODE: u16 = 1;
pub const SS_CODE: u16 = 2;
pub const FIL_NEG_CODE: u16 = 3;
pub const SKIP_CODE: u16 = 4;
pub const MSKIP_CODE: u16 = 5;

pub const BOX_CODE: u16 = 0;
pub const COPY_CODE: u16 = 1;
pub const LAST_BOX_CODE: u16 = 2;
pub const VSPLIT_CODE: u16 = 3;
pub const VTOP_CODE: u16 = 4;

pub const A_LEADERS: u16 = 100;
pub const C_LEADERS: u16 = 101;
pub const X_LEADERS: u16 = 102;

pub const GLUE_NODE_TYPE: u16 = 10;
pub const KERN_NODE_TYPE: u16 = 11;
pub const PENALTY_NODE_TYPE: u16 = 12;

pub const ORD_NOAD: u16 = 16;
pub const OP_NOAD: u16 = 17;
pub const BIN_NOAD: u16 = 18;
pub const REL_NOAD: u16 = 19;
pub const OPEN_NOAD: u16 = 20;
pub const CLOSE_NOAD: u16 = 21;
pub const PUNCT_NOAD: u16 = 22;
pub const INNER_NOAD: u16 = 23;
pub const RADICAL_NOAD: u16 = 24;
pub const FRACTION_NOAD: u16 = 25;
pub const UNDER_NOAD: u16 = 26;
pub const OVER_NOAD: u16 = 27;
pub const ACCENT_NOAD: u16 = 28;
pub const VCENTER_NOAD: u16 = 29;
pub const LEFT_NOAD: u16 = 30;
pub const RIGHT_NOAD: u16 = 31;

pub const LIMITS: u16 = 1;
pub const NO_LIMITS: u16 = 2;

pub const DISPLAY_STYLE: u16 = 0;
pub const TEXT_STYLE: u16 = 2;
pub const SCRIPT_STYLE: u16 = 4;
pub const SCRIPT_SCRIPT_STYLE: u16 = 6;

pub const TEXT_SIZE: u16 = 0;
pub const SCRIPT_SIZE: u16 = 16;
pub const SCRIPT_SCRIPT_SIZE: u16 = 32;

pub const ABOVE_CODE: u16 = 0;
pub const OVER_CODE: u16 = 1;
pub const ATOP_CODE: u16 = 2;
pub const DELIMITED_CODE: u16 = 3;

pub const CHAR_DEF_CODE: u16 = 0;
pub const MATH_CHAR_DEF_CODE: u16 = 1;
pub const COUNT_DEF_CODE: u16 = 2;
pub const DIMEN_DEF_CODE: u16 = 3;
pub const SKIP_DEF_CODE: u16 = 4;
pub const MU_SKIP_DEF_CODE: u16 = 5;
pub const TOKS_DEF_CODE: u16 = 6;

pub const SHOW_CODE: u16 = 0;
pub const SHOW_BOX_CODE: u16 = 1;
pub const SHOW_THE_CODE: u16 = 2;
pub const SHOW_LISTS_CODE: u16 = 3;

pub const OPEN_NODE: u16 = 0;
pub const WRITE_NODE: u16 = 1;
pub const CLOSE_NODE: u16 = 2;
pub const SPECIAL_NODE: u16 = 3;
pub const IMMEDIATE_CODE: u16 = 4;
pub const SET_LANGUAGE_CODE: u16 = 5;

/// §§289–290: Packed token constants (`cs_token_flag = 4095`).
pub const CS_TOKEN_FLAG: u16 = 0o7777; // 4095
pub const LEFT_BRACE_TOKEN: u16 = 0o0400; // (1 << 8)
pub const LEFT_BRACE_LIMIT: u16 = 0o1000; // (2 << 8)
pub const RIGHT_BRACE_TOKEN: u16 = 0o1000; // (2 << 8)
pub const RIGHT_BRACE_LIMIT: u16 = 0o1400; // (3 << 8)
pub const MATH_SHIFT_TOKEN: u16 = 0o1400; // (3 << 8)
pub const TAB_TOKEN: u16 = 0o2000; // (4 << 8)
pub const OUT_PARAM_TOKEN: u16 = 0o2400; // (5 << 8)
pub const SPACE_TOKEN: u16 = 0o5040; // (10 << 8) + b' '
pub const LETTER_TOKEN: u16 = 0o5400; // (11 << 8)
pub const OTHER_TOKEN: u16 = 0o6000; // (12 << 8)
pub const MATCH_TOKEN: u16 = 0o6400; // (13 << 8)
pub const END_MATCH_TOKEN: u16 = 0o7000; // (14 << 8)

/// §§289–290: A packed halfword token in a token list or `cur_tok`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Token(pub u16);

impl Token {
    pub const fn from_cmd_chr(cmd: u8, chr: u8) -> Self {
        Self(((cmd as u16) << 8) | (chr as u16))
    }

    pub const fn from_cs(cs: u16) -> Self {
        Self(CS_TOKEN_FLAG + cs)
    }

    pub const fn is_cs(self) -> bool {
        self.0 >= CS_TOKEN_FLAG
    }

    pub const fn cs(self) -> Option<u16> {
        if self.0 >= CS_TOKEN_FLAG {
            Some(self.0 - CS_TOKEN_FLAG)
        } else {
            None
        }
    }

    pub const fn cmd_chr(self) -> Option<(u8, u8)> {
        if self.0 < CS_TOKEN_FLAG {
            Some(((self.0 >> 8) as u8, (self.0 & 0xff) as u8))
        } else {
            None
        }
    }

    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// §§212–213: One entry on the semantic nest (`list_state_record`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListStateRecord {
    pub mode_field: i32,
    pub head_field: MemPtr,
    pub tail_field: MemPtr,
    pub pg_field: i32,
    pub ml_field: i32,
    /// Raw 32-bit `aux_field` (`prev_depth` in vmode, `(space_factor, clang)` in hmode, `incompleat_noad` in mmode).
    pub aux_int: i32,
}

impl Default for ListStateRecord {
    fn default() -> Self {
        Self {
            mode_field: VMODE,
            head_field: MemPtr::NULL,
            tail_field: MemPtr::NULL,
            pg_field: 0,
            ml_field: 0,
            aux_int: IGNORE_DEPTH,
        }
    }
}

impl ListStateRecord {
    pub const fn prev_depth(&self) -> Scaled {
        match Scaled::from_raw(self.aux_int) {
            Some(s) => s,
            None => Scaled::ZERO,
        }
    }

    pub fn set_prev_depth(&mut self, s: Scaled) {
        self.aux_int = s.raw();
    }

    pub fn space_factor(&self) -> u16 {
        let u = u32::from_be_bytes(self.aux_int.to_be_bytes());
        (u >> 16) as u16
    }

    pub fn set_space_factor(&mut self, sf: u16) {
        let u = u32::from_be_bytes(self.aux_int.to_be_bytes());
        let next = (u & 0xffff) | (u32::from(sf) << 16);
        self.aux_int = i32::from_be_bytes(next.to_be_bytes());
    }

    pub fn clang(&self) -> u16 {
        let u = u32::from_be_bytes(self.aux_int.to_be_bytes());
        (u & 0xffff) as u16
    }

    pub fn set_clang(&mut self, clang: u16) {
        let u = u32::from_be_bytes(self.aux_int.to_be_bytes());
        let next = (u & 0xffff_0000) | u32::from(clang);
        self.aux_int = i32::from_be_bytes(next.to_be_bytes());
    }

    pub const fn incompleat_noad(&self) -> i32 {
        self.aux_int
    }

    pub fn set_incompleat_noad(&mut self, val: i32) {
        self.aux_int = val;
    }
}

/// §§211–219: The semantic nest.
#[derive(Clone, Debug)]
pub struct SemanticNest {
    pub nest: Vec<ListStateRecord>,
    pub nest_ptr: usize,
    pub max_nest_stack: usize,
    pub nest_size: usize,
    pub cur_list: ListStateRecord,
    pub shown_mode: i32,
}

impl SemanticNest {
    /// §215: Initialize the semantic nest at `vmode` with `head = tail = contrib_head`.
    pub fn new(nest_size: usize, contrib_head: MemPtr) -> Self {
        let cur_list = ListStateRecord {
            mode_field: VMODE,
            head_field: contrib_head,
            tail_field: contrib_head,
            pg_field: 0,
            ml_field: 0,
            aux_int: IGNORE_DEPTH,
        };
        Self {
            nest: vec![ListStateRecord::default(); nest_size + 1],
            nest_ptr: 0,
            max_nest_stack: 0,
            nest_size,
            cur_list,
            shown_mode: 0,
        }
    }

    /// §211: `print_mode(m)`.
    pub fn print_mode(io: &mut IoState, m: i32) {
        let denom = i32::from(cmd::MAX_COMMAND) + 1;
        match m.cmp(&0) {
            std::cmp::Ordering::Greater => match m / denom {
                0 => io.print_str(pool_str("vertical")),
                1 => io.print_str(pool_str("horizontal")),
                2 => io.print_str(pool_str("display math")),
                _ => {}
            },
            std::cmp::Ordering::Equal => io.print_str(pool_str("no")),
            std::cmp::Ordering::Less => match (-m) / denom {
                0 => io.print_str(pool_str("internal vertical")),
                1 => io.print_str(pool_str("restricted horizontal")),
                2 => io.print_str(pool_str("math")),
                _ => {}
            },
        }
        io.print_str(pool_str(" mode"));
    }

    /// §216: `push_nest`.
    pub fn push_nest(
        &mut self,
        io: &mut IoState,
        memory: &mut Memory,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        if self.nest_ptr > self.max_nest_stack {
            self.max_nest_stack = self.nest_ptr;
            if self.nest_ptr == self.nest_size {
                return Err(io.overflow_with_hooks(
                    pool_str("semantic nest size"),
                    self.nest_size as i32,
                    hooks,
                ));
            }
        }
        self.nest[self.nest_ptr] = self.cur_list;
        self.nest_ptr += 1;
        let head = memory.get_avail().map_err(|e| match e {
            MemoryError::Exhausted { capacity } => {
                io.overflow_with_hooks(StrNum::MAIN_MEMORY_SIZE, capacity as i32, hooks)
            }
            _ => io.confusion_with_hooks(pool_str("push_nest"), hooks),
        })?;
        self.cur_list.head_field = head;
        self.cur_list.tail_field = head;
        self.cur_list.pg_field = 0;
        self.cur_list.ml_field = io.line;
        Ok(())
    }

    /// §217: `pop_nest`.
    pub fn pop_nest(&mut self, memory: &mut Memory) -> Result<(), MemoryError> {
        memory.free_avail(self.cur_list.head_field)?;
        self.nest_ptr -= 1;
        self.cur_list = self.nest[self.nest_ptr];
        Ok(())
    }

    /// §214: `tail_append(p)`.
    pub fn tail_append(&mut self, memory: &mut Memory, p: MemPtr) -> Result<(), MemoryError> {
        memory.set_head_link(self.cur_list.tail_field, p)?;
        self.cur_list.tail_field = p;
        Ok(())
    }
}

/// §§222, 224, 230, 236, 247: Address layout for `eqtb` parameterized by `hash_size`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EqtbLayout {
    pub hash_size: usize,
    pub active_base: usize,
    pub single_base: usize,
    pub null_cs: usize,
    pub hash_base: usize,
    pub frozen_control_sequence: usize,
    pub frozen_protection: usize,
    pub frozen_cr: usize,
    pub frozen_end_group: usize,
    pub frozen_right: usize,
    pub frozen_fi: usize,
    pub frozen_end_template: usize,
    pub frozen_endv: usize,
    pub frozen_relax: usize,
    pub end_write: usize,
    pub frozen_dont_expand: usize,
    pub frozen_null_font: usize,
    pub font_id_base: usize,
    pub undefined_control_sequence: usize,
    pub glue_base: usize,
    pub skip_base: usize,
    pub mu_skip_base: usize,
    pub local_base: usize,
    pub par_shape_loc: usize,
    pub output_routine_loc: usize,
    pub every_par_loc: usize,
    pub every_math_loc: usize,
    pub every_display_loc: usize,
    pub every_hbox_loc: usize,
    pub every_vbox_loc: usize,
    pub every_job_loc: usize,
    pub every_cr_loc: usize,
    pub err_help_loc: usize,
    pub toks_base: usize,
    pub box_base: usize,
    pub cur_font_loc: usize,
    pub math_font_base: usize,
    pub cat_code_base: usize,
    pub lc_code_base: usize,
    pub uc_code_base: usize,
    pub sf_code_base: usize,
    pub math_code_base: usize,
    pub int_base: usize,
    pub count_base: usize,
    pub del_code_base: usize,
    pub dimen_base: usize,
    pub scaled_base: usize,
    pub eqtb_size: usize,
}

impl EqtbLayout {
    pub const fn new(hash_size: usize) -> Self {
        let active_base = 1;
        let single_base = active_base + 256;
        let null_cs = single_base + 256;
        let hash_base = null_cs + 1;
        let frozen_control_sequence = hash_base + hash_size;
        let frozen_protection = frozen_control_sequence;
        let frozen_cr = frozen_control_sequence + 1;
        let frozen_end_group = frozen_control_sequence + 2;
        let frozen_right = frozen_control_sequence + 3;
        let frozen_fi = frozen_control_sequence + 4;
        let frozen_end_template = frozen_control_sequence + 5;
        let frozen_endv = frozen_control_sequence + 6;
        let frozen_relax = frozen_control_sequence + 7;
        let end_write = frozen_control_sequence + 8;
        let frozen_dont_expand = frozen_control_sequence + 9;
        let frozen_null_font = frozen_control_sequence + 10;
        let font_id_base = frozen_null_font - (FONT_BASE as usize);
        let undefined_control_sequence = frozen_null_font + 257;
        let glue_base = undefined_control_sequence + 1;
        let skip_base = glue_base + GLUE_PARS;
        let mu_skip_base = skip_base + 256;
        let local_base = mu_skip_base + 256;
        let par_shape_loc = local_base;
        let output_routine_loc = local_base + 1;
        let every_par_loc = local_base + 2;
        let every_math_loc = local_base + 3;
        let every_display_loc = local_base + 4;
        let every_hbox_loc = local_base + 5;
        let every_vbox_loc = local_base + 6;
        let every_job_loc = local_base + 7;
        let every_cr_loc = local_base + 8;
        let err_help_loc = local_base + 9;
        let toks_base = local_base + 10;
        let box_base = toks_base + 256;
        let cur_font_loc = box_base + 256;
        let math_font_base = cur_font_loc + 1;
        let cat_code_base = math_font_base + 48;
        let lc_code_base = cat_code_base + 256;
        let uc_code_base = lc_code_base + 256;
        let sf_code_base = uc_code_base + 256;
        let math_code_base = sf_code_base + 256;
        let int_base = math_code_base + 256;
        let count_base = int_base + INT_PARS;
        let del_code_base = count_base + 256;
        let dimen_base = del_code_base + 256;
        let scaled_base = dimen_base + DIMEN_PARS;
        let eqtb_size = scaled_base + 255;
        Self {
            hash_size,
            active_base,
            single_base,
            null_cs,
            hash_base,
            frozen_control_sequence,
            frozen_protection,
            frozen_cr,
            frozen_end_group,
            frozen_right,
            frozen_fi,
            frozen_end_template,
            frozen_endv,
            frozen_relax,
            end_write,
            frozen_dont_expand,
            frozen_null_font,
            font_id_base,
            undefined_control_sequence,
            glue_base,
            skip_base,
            mu_skip_base,
            local_base,
            par_shape_loc,
            output_routine_loc,
            every_par_loc,
            every_math_loc,
            every_display_loc,
            every_hbox_loc,
            every_vbox_loc,
            every_job_loc,
            every_cr_loc,
            err_help_loc,
            toks_base,
            box_base,
            cur_font_loc,
            math_font_base,
            cat_code_base,
            lc_code_base,
            uc_code_base,
            sf_code_base,
            math_code_base,
            int_base,
            count_base,
            del_code_base,
            dimen_base,
            scaled_base,
            eqtb_size,
        }
    }
}

/// §221: One word in `eqtb` or `save_stack` (either `(b0=eq_type/save_type, b1=eq_level/save_level, rh=equiv/save_index)` or fullword `int`/`sc`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EqWord(pub u32);

impl EqWord {
    pub const fn from_hh(b0: u8, b1: u8, rh: u16) -> Self {
        let [hi, lo] = rh.to_be_bytes();
        Self(u32::from_be_bytes([b0, b1, hi, lo]))
    }

    pub const fn from_int(val: i32) -> Self {
        Self(u32::from_be_bytes(val.to_be_bytes()))
    }

    pub const fn b0(self) -> u8 {
        self.0.to_be_bytes()[0]
    }

    pub const fn b1(self) -> u8 {
        self.0.to_be_bytes()[1]
    }

    pub const fn rh(self) -> u16 {
        (self.0 & 0xffff) as u16
    }

    pub fn set_b0(&mut self, val: u8) {
        let mut b = self.0.to_be_bytes();
        b[0] = val;
        self.0 = u32::from_be_bytes(b);
    }

    pub fn set_b1(&mut self, val: u8) {
        let mut b = self.0.to_be_bytes();
        b[1] = val;
        self.0 = u32::from_be_bytes(b);
    }

    pub fn set_rh(&mut self, val: u16) {
        self.0 = (self.0 & 0xffff_0000) | u32::from(val);
    }

    pub const fn int(self) -> i32 {
        i32::from_be_bytes(self.0.to_be_bytes())
    }

    pub fn set_int(&mut self, val: i32) {
        self.0 = u32::from_be_bytes(val.to_be_bytes());
    }
}

/// §§220–255: The table of equivalents (`eqtb` and `xeq_level`).
#[derive(Clone, Debug)]
pub struct Eqtb {
    pub layout: EqtbLayout,
    pub words: Vec<EqWord>,
    pub xeq_level: Vec<u8>,
}

impl Eqtb {
    /// §§222, 228, 232, 240, 250, 254: Initialize `eqtb` and `xeq_level` to pristine INITEX values,
    /// updating `glue_ref_count(zero_glue)` in `memory` and syncing cached parameters with `io`.
    pub fn new(hash_size: usize, memory: &mut Memory, io: &mut IoState) -> Self {
        let layout = EqtbLayout::new(hash_size);
        let mut words = vec![EqWord::default(); layout.eqtb_size + 1];
        let xeq_len = layout.eqtb_size - layout.int_base + 1;
        let xeq_level = vec![LEVEL_ONE; xeq_len];

        // §222: Region 1 and 2 initialization.
        let undef = EqWord::from_hh(cmd::UNDEFINED_CS, LEVEL_ZERO, MemPtr::NULL.address());
        words[layout.undefined_control_sequence] = undef;
        for slot in &mut words[layout.active_base..layout.undefined_control_sequence] {
            *slot = undef;
        }

        // §228: Region 3 initialization (glue parameters and skip/muskip registers).
        let zero_glue = memory.zero_glue();
        let glue_init = EqWord::from_hh(cmd::GLUE_REF, LEVEL_ONE, zero_glue.address());
        for slot in &mut words[layout.glue_base..layout.local_base] {
            *slot = glue_init;
        }
        let added_refs = (layout.local_base - layout.glue_base) as u16;
        let cur_rc = memory.glue_ref_count(zero_glue).unwrap_or(0);
        let _ = memory.set_glue_ref_count(zero_glue, cur_rc.saturating_add(added_refs));

        // §232: Region 4 initialization.
        words[layout.par_shape_loc] =
            EqWord::from_hh(cmd::SHAPE_REF, LEVEL_ONE, MemPtr::NULL.address());
        for slot in &mut words[layout.output_routine_loc..=layout.toks_base + 255] {
            *slot = undef;
        }
        let box_init = EqWord::from_hh(cmd::BOX_REF, LEVEL_ONE, MemPtr::NULL.address());
        for slot in &mut words[layout.box_base..=layout.box_base + 255] {
            *slot = box_init;
        }
        let font_init = EqWord::from_hh(cmd::DATA, LEVEL_ONE, NULL_FONT);
        words[layout.cur_font_loc] = font_init;
        for slot in &mut words[layout.math_font_base..=layout.math_font_base + 47] {
            *slot = font_init;
        }
        let data_zero = EqWord::from_hh(cmd::DATA, LEVEL_ONE, 0);
        for slot in &mut words[layout.cat_code_base..layout.int_base] {
            *slot = data_zero;
        }

        let mut eqtb = Self {
            layout,
            words,
            xeq_level,
        };

        for k in 0..=255_u8 {
            eqtb.set_cat_code(k, u16::from(cmd::OTHER_CHAR));
            eqtb.set_math_code(k, u16::from(k));
            eqtb.set_sf_code(k, 1000);
        }
        eqtb.set_cat_code(CARRIAGE_RETURN, u16::from(cmd::CAR_RET));
        eqtb.set_cat_code(b' ', u16::from(cmd::SPACER));
        eqtb.set_cat_code(b'\\', u16::from(cmd::ESCAPE));
        eqtb.set_cat_code(b'%', u16::from(cmd::COMMENT));
        eqtb.set_cat_code(INVALID_CODE, u16::from(cmd::INVALID_CHAR));
        eqtb.set_cat_code(NULL_CODE, u16::from(cmd::IGNORE));
        for k in b'0'..=b'9' {
            eqtb.set_math_code(k, u16::from(k) + VAR_CODE);
        }
        for k in b'A'..=b'Z' {
            let lower = k + b'a' - b'A';
            eqtb.set_cat_code(k, u16::from(cmd::LETTER));
            eqtb.set_cat_code(lower, u16::from(cmd::LETTER));
            eqtb.set_math_code(k, u16::from(k) + VAR_CODE + 0x100);
            eqtb.set_math_code(lower, u16::from(lower) + VAR_CODE + 0x100);
            eqtb.set_lc_code(k, u16::from(lower));
            eqtb.set_lc_code(lower, u16::from(lower));
            eqtb.set_uc_code(k, u16::from(k));
            eqtb.set_uc_code(lower, u16::from(k));
            eqtb.set_sf_code(k, 999);
        }

        // §240: Region 5 initialization.
        for word in &mut eqtb.words[layout.int_base..layout.del_code_base] {
            word.set_int(0);
        }
        eqtb.set_int_par(MAG_CODE, 1000);
        eqtb.set_int_par(TOLERANCE_CODE, 10000);
        eqtb.set_int_par(HANG_AFTER_CODE, 1);
        eqtb.set_int_par(MAX_DEAD_CYCLES_CODE, 25);
        eqtb.set_int_par(ESCAPE_CHAR_CODE, i32::from(b'\\'));
        eqtb.set_int_par(END_LINE_CHAR_CODE, i32::from(CARRIAGE_RETURN));
        for k in 0..=255_u8 {
            eqtb.set_del_code(k, -1);
        }
        eqtb.set_del_code(b'.', 0);

        // §250: Region 6 initialization.
        for word in &mut eqtb.words[layout.dimen_base..=layout.eqtb_size] {
            word.set_int(0);
        }

        // §258: frozen_dont_expand.
        eqtb.words[layout.frozen_dont_expand] =
            EqWord::from_hh(cmd::DONT_EXPAND, LEVEL_ZERO, MemPtr::NULL.address());

        eqtb.sync_io_params(io);
        eqtb
    }

    pub fn eq_type(&self, p: usize) -> u8 {
        self.words[p].b0()
    }

    pub fn set_eq_type(&mut self, p: usize, t: u8) {
        self.words[p].set_b0(t);
    }

    pub fn eq_level(&self, p: usize) -> u8 {
        if p >= self.layout.int_base {
            self.xeq_level[p - self.layout.int_base]
        } else {
            self.words[p].b1()
        }
    }

    pub fn set_eq_level(&mut self, p: usize, l: u8) {
        if p >= self.layout.int_base {
            self.xeq_level[p - self.layout.int_base] = l;
        } else {
            self.words[p].set_b1(l);
        }
    }

    pub fn equiv(&self, p: usize) -> u16 {
        self.words[p].rh()
    }

    pub fn set_equiv(&mut self, p: usize, e: u16) {
        self.words[p].set_rh(e);
    }

    pub fn int(&self, p: usize) -> i32 {
        self.words[p].int()
    }

    pub fn set_int(&mut self, p: usize, val: i32) {
        self.words[p].set_int(val);
    }

    pub fn scaled(&self, p: usize) -> Scaled {
        Scaled::from_raw(self.words[p].int()).unwrap_or(Scaled::ZERO)
    }

    pub fn set_scaled(&mut self, p: usize, val: Scaled) {
        self.words[p].set_int(val.raw());
    }

    pub fn cat_code(&self, c: u8) -> u8 {
        self.equiv(self.layout.cat_code_base + usize::from(c)) as u8
    }

    pub fn set_cat_code(&mut self, c: u8, val: u16) {
        self.set_equiv(self.layout.cat_code_base + usize::from(c), val);
    }

    pub fn lc_code(&self, c: u8) -> u16 {
        self.equiv(self.layout.lc_code_base + usize::from(c))
    }

    pub fn set_lc_code(&mut self, c: u8, val: u16) {
        self.set_equiv(self.layout.lc_code_base + usize::from(c), val);
    }

    pub fn uc_code(&self, c: u8) -> u16 {
        self.equiv(self.layout.uc_code_base + usize::from(c))
    }

    pub fn set_uc_code(&mut self, c: u8, val: u16) {
        self.set_equiv(self.layout.uc_code_base + usize::from(c), val);
    }

    pub fn sf_code(&self, c: u8) -> u16 {
        self.equiv(self.layout.sf_code_base + usize::from(c))
    }

    pub fn set_sf_code(&mut self, c: u8, val: u16) {
        self.set_equiv(self.layout.sf_code_base + usize::from(c), val);
    }

    pub fn math_code(&self, c: u8) -> u16 {
        self.equiv(self.layout.math_code_base + usize::from(c))
    }

    pub fn set_math_code(&mut self, c: u8, val: u16) {
        self.set_equiv(self.layout.math_code_base + usize::from(c), val);
    }

    pub fn del_code(&self, c: u8) -> i32 {
        self.int(self.layout.del_code_base + usize::from(c))
    }

    pub fn set_del_code(&mut self, c: u8, val: i32) {
        self.set_int(self.layout.del_code_base + usize::from(c), val);
    }

    pub fn int_par(&self, code: usize) -> i32 {
        self.int(self.layout.int_base + code)
    }

    pub fn set_int_par(&mut self, code: usize, val: i32) {
        self.set_int(self.layout.int_base + code, val);
    }

    pub fn count(&self, reg: u8) -> i32 {
        self.int(self.layout.count_base + usize::from(reg))
    }

    pub fn set_count(&mut self, reg: u8, val: i32) {
        self.set_int(self.layout.count_base + usize::from(reg), val);
    }

    pub fn dimen_par(&self, code: usize) -> Scaled {
        self.scaled(self.layout.dimen_base + code)
    }

    pub fn set_dimen_par(&mut self, code: usize, val: Scaled) {
        self.set_scaled(self.layout.dimen_base + code, val);
    }

    pub fn dimen(&self, reg: u8) -> Scaled {
        self.scaled(self.layout.scaled_base + usize::from(reg))
    }

    pub fn set_dimen(&mut self, reg: u8, val: Scaled) {
        self.set_scaled(self.layout.scaled_base + usize::from(reg), val);
    }

    pub fn glue_par(&self, code: usize) -> MemPtr {
        MemPtr::from_address(self.equiv(self.layout.glue_base + code))
    }

    pub fn skip(&self, reg: u8) -> MemPtr {
        MemPtr::from_address(self.equiv(self.layout.skip_base + usize::from(reg)))
    }

    pub fn mu_skip(&self, reg: u8) -> MemPtr {
        MemPtr::from_address(self.equiv(self.layout.mu_skip_base + usize::from(reg)))
    }

    pub fn toks(&self, reg: u8) -> MemPtr {
        MemPtr::from_address(self.equiv(self.layout.toks_base + usize::from(reg)))
    }

    /// Synchronize `IoState`'s cached parameters from `eqtb` region 5.
    pub fn sync_io_params(&self, io: &mut IoState) {
        io.escape_char = self.int_par(ESCAPE_CHAR_CODE);
        io.end_line_char = self.int_par(END_LINE_CHAR_CODE);
        io.new_line_char = self.int_par(NEW_LINE_CHAR_CODE);
        io.tracing_online = self.int_par(TRACING_ONLINE_CODE);
        io.pausing = self.int_par(PAUSING_CODE);
        io.time = self.int_par(TIME_CODE);
        io.day = self.int_par(DAY_CODE);
        io.month = self.int_par(MONTH_CODE);
        io.year = self.int_par(YEAR_CODE);
        io.use_err_help = self.equiv(self.layout.err_help_loc) != 0;
    }

    /// §241: `fix_date_and_time` across both `IoState` and `eqtb`.
    pub fn fix_date_and_time(&mut self, io: &mut IoState) {
        io.fix_date_and_time();
        self.set_int_par(TIME_CODE, io.time);
        self.set_int_par(DAY_CODE, io.day);
        self.set_int_par(MONTH_CODE, io.month);
        self.set_int_par(YEAR_CODE, io.year);
    }

    /// §225: `print_skip_param(n)`.
    pub fn print_skip_param(io: &mut IoState, n: i32) {
        let name = match n as usize {
            LINE_SKIP_CODE => "lineskip",
            BASELINE_SKIP_CODE => "baselineskip",
            PAR_SKIP_CODE => "parskip",
            ABOVE_DISPLAY_SKIP_CODE => "abovedisplayskip",
            BELOW_DISPLAY_SKIP_CODE => "belowdisplayskip",
            ABOVE_DISPLAY_SHORT_SKIP_CODE => "abovedisplayshortskip",
            BELOW_DISPLAY_SHORT_SKIP_CODE => "belowdisplayshortskip",
            LEFT_SKIP_CODE => "leftskip",
            RIGHT_SKIP_CODE => "rightskip",
            TOP_SKIP_CODE => "topskip",
            SPLIT_TOP_SKIP_CODE => "splittopskip",
            TAB_SKIP_CODE => "tabskip",
            SPACE_SKIP_CODE => "spaceskip",
            XSPACE_SKIP_CODE => "xspaceskip",
            PAR_FILL_SKIP_CODE => "parfillskip",
            THIN_MU_SKIP_CODE => "thinmuskip",
            MED_MU_SKIP_CODE => "medmuskip",
            THICK_MU_SKIP_CODE => "thickmuskip",
            _ => {
                io.print_str(pool_str("[unknown glue parameter!]"));
                return;
            }
        };
        io.print_esc(pool_str(name));
    }

    /// §237: `print_param(n)`.
    pub fn print_param(io: &mut IoState, n: i32) {
        let name = match n as usize {
            PRETOLERANCE_CODE => "pretolerance",
            TOLERANCE_CODE => "tolerance",
            LINE_PENALTY_CODE => "linepenalty",
            HYPHEN_PENALTY_CODE => "hyphenpenalty",
            EX_HYPHEN_PENALTY_CODE => "exhyphenpenalty",
            CLUB_PENALTY_CODE => "clubpenalty",
            WIDOW_PENALTY_CODE => "widowpenalty",
            DISPLAY_WIDOW_PENALTY_CODE => "displaywidowpenalty",
            BROKEN_PENALTY_CODE => "brokenpenalty",
            BIN_OP_PENALTY_CODE => "binoppenalty",
            REL_PENALTY_CODE => "relpenalty",
            PRE_DISPLAY_PENALTY_CODE => "predisplaypenalty",
            POST_DISPLAY_PENALTY_CODE => "postdisplaypenalty",
            INTER_LINE_PENALTY_CODE => "interlinepenalty",
            DOUBLE_HYPHEN_DEMERITS_CODE => "doublehyphendemerits",
            FINAL_HYPHEN_DEMERITS_CODE => "finalhyphendemerits",
            ADJ_DEMERITS_CODE => "adjdemerits",
            MAG_CODE => "mag",
            DELIMITER_FACTOR_CODE => "delimiterfactor",
            LOOSENESS_CODE => "looseness",
            TIME_CODE => "time",
            DAY_CODE => "day",
            MONTH_CODE => "month",
            YEAR_CODE => "year",
            SHOW_BOX_BREADTH_CODE => "showboxbreadth",
            SHOW_BOX_DEPTH_CODE => "showboxdepth",
            HBADNESS_CODE => "hbadness",
            VBADNESS_CODE => "vbadness",
            PAUSING_CODE => "pausing",
            TRACING_ONLINE_CODE => "tracingonline",
            TRACING_MACROS_CODE => "tracingmacros",
            TRACING_STATS_CODE => "tracingstats",
            TRACING_PARAGRAPHS_CODE => "tracingparagraphs",
            TRACING_PAGES_CODE => "tracingpages",
            TRACING_OUTPUT_CODE => "tracingoutput",
            TRACING_LOST_CHARS_CODE => "tracinglostchars",
            TRACING_COMMANDS_CODE => "tracingcommands",
            TRACING_RESTORES_CODE => "tracingrestores",
            UC_HYPH_CODE => "uchyph",
            OUTPUT_PENALTY_CODE => "outputpenalty",
            MAX_DEAD_CYCLES_CODE => "maxdeadcycles",
            HANG_AFTER_CODE => "hangafter",
            FLOATING_PENALTY_CODE => "floatingpenalty",
            GLOBAL_DEFS_CODE => "globaldefs",
            CUR_FAM_CODE => "fam",
            ESCAPE_CHAR_CODE => "escapechar",
            DEFAULT_HYPHEN_CHAR_CODE => "defaulthyphenchar",
            DEFAULT_SKEW_CHAR_CODE => "defaultskewchar",
            END_LINE_CHAR_CODE => "endlinechar",
            NEW_LINE_CHAR_CODE => "newlinechar",
            LANGUAGE_CODE => "language",
            LEFT_HYPHEN_MIN_CODE => "lefthyphenmin",
            RIGHT_HYPHEN_MIN_CODE => "righthyphenmin",
            HOLDING_INSERTS_CODE => "holdinginserts",
            ERROR_CONTEXT_LINES_CODE => "errorcontextlines",
            _ => {
                io.print_str(pool_str("[unknown integer parameter!]"));
                return;
            }
        };
        io.print_esc(pool_str(name));
    }

    /// §247: `print_length_param(n)`.
    pub fn print_length_param(io: &mut IoState, n: i32) {
        let name = match n as usize {
            PAR_INDENT_CODE => "parindent",
            MATH_SURROUND_CODE => "mathsurround",
            LINE_SKIP_LIMIT_CODE => "lineskiplimit",
            HSIZE_CODE => "hsize",
            VSIZE_CODE => "vsize",
            MAX_DEPTH_CODE => "maxdepth",
            SPLIT_MAX_DEPTH_CODE => "splitmaxdepth",
            BOX_MAX_DEPTH_CODE => "boxmaxdepth",
            HFUZZ_CODE => "hfuzz",
            VFUZZ_CODE => "vfuzz",
            DELIMITER_SHORTFALL_CODE => "delimitershortfall",
            NULL_DELIMITER_SPACE_CODE => "nulldelimiterspace",
            SCRIPT_SPACE_CODE => "scriptspace",
            PRE_DISPLAY_SIZE_CODE => "predisplaysize",
            DISPLAY_WIDTH_CODE => "displaywidth",
            DISPLAY_INDENT_CODE => "displayindent",
            OVERFULL_RULE_CODE => "overfullrule",
            HANG_INDENT_CODE => "hangindent",
            H_OFFSET_CODE => "hoffset",
            V_OFFSET_CODE => "voffset",
            EMERGENCY_STRETCH_CODE => "emergencystretch",
            _ => {
                io.print_str(pool_str("[unknown dimen parameter!]"));
                return;
            }
        };
        io.print_esc(pool_str(name));
    }

    /// §255: Search `eqtb` for equivalents equal to `p`.
    pub fn search_eqtb(&self, io: &mut IoState, p: u16) {
        for q in self.layout.active_base..=self.layout.box_base + 255 {
            if self.equiv(q) == p {
                io.print_nl(pool_str("EQUIV("));
                io.print_int(q as i32);
                io.print_char(b')');
            }
        }
    }
}

/// §256: One word in the `hash` array (`lh = next`, `rh = text`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HashEntry {
    pub next: u16,
    pub text: StrNum,
}

/// §§256–267: Coalesced-list hash table for multiletter control sequences.
#[derive(Clone, Debug)]
pub struct HashTable {
    pub entries: Vec<HashEntry>,
    pub hash_used: usize,
    pub no_new_control_sequence: bool,
    pub cs_count: i32,
    pub par_loc: u16,
    pub par_token: Token,
    pub write_loc: u16,
}

impl HashTable {
    /// §§257–258: Initialize the hash table and set `text(frozen_dont_expand) = "notexpanded:"`.
    pub fn new(layout: &EqtbLayout) -> Self {
        let mut entries = vec![HashEntry::default(); layout.undefined_control_sequence];
        entries[layout.frozen_dont_expand].text = pool_str("notexpanded:");
        Self {
            entries,
            hash_used: layout.frozen_control_sequence,
            no_new_control_sequence: true,
            cs_count: 0,
            par_loc: 0,
            par_token: Token::default(),
            write_loc: 0,
        }
    }

    pub fn next(&self, p: usize) -> u16 {
        self.entries[p].next
    }

    pub fn set_next(&mut self, p: usize, val: u16) {
        self.entries[p].next = val;
    }

    pub fn text(&self, p: usize) -> StrNum {
        self.entries[p].text
    }

    pub fn set_text(&mut self, p: usize, val: StrNum) {
        self.entries[p].text = val;
    }

    pub fn font_id_text(&self, layout: &EqtbLayout, f: u16) -> StrNum {
        self.text(layout.font_id_base + usize::from(f))
    }

    pub fn set_font_id_text(&mut self, layout: &EqtbLayout, f: u16, s: StrNum) {
        self.set_text(layout.font_id_base + usize::from(f), s);
    }

    /// §§259–261: `id_lookup(j, l)` searches or inserts `buffer[j .. j + l]`.
    /// §§259–261: `id_lookup(j, l)` searches or inserts `buffer[j .. j + l]`.
    pub fn id_lookup(
        &mut self,
        io: &mut IoState,
        layout: &EqtbLayout,
        j: usize,
        l: usize,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<u16, Termination> {
        match self.try_id_lookup(io, layout, j, l) {
            Ok(cs) => Ok(cs),
            Err((s, n)) => Err(io.overflow_with_hooks(s, n, hooks)),
        }
    }

    /// §§259–261: `id_lookup` returning `Err((overflow_str, overflow_val))` on capacity exhaustion
    /// so callers can invoke `io.overflow_with_hooks` after releasing `&mut self`.
    pub fn try_id_lookup(
        &mut self,
        io: &mut IoState,
        layout: &EqtbLayout,
        j: usize,
        l: usize,
    ) -> Result<u16, (StrNum, i32)> {
        // §261: Compute the hash code h.
        let hash_prime = io.config.hash_prime;
        let mut h = usize::from(io.buffer[j]);
        for &character in &io.buffer[(j + 1)..(j + l)] {
            h = h + h + usize::from(character);
            while h >= hash_prime {
                h -= hash_prime;
            }
        }
        let mut p = h + layout.hash_base;
        loop {
            let txt = self.text(p);
            if txt.0 > 0 && io.length(txt) == l && io.str_eq_buf(txt, j) {
                return Ok(p as u16);
            }
            if self.next(p) == 0 {
                if self.no_new_control_sequence {
                    p = layout.undefined_control_sequence;
                } else {
                    // §260: Insert a new control sequence after p, then make p point to it.
                    if self.text(p).0 > 0 {
                        loop {
                            if self.hash_used == layout.hash_base {
                                return Err((pool_str("hash size"), layout.hash_size as i32));
                            }
                            self.hash_used -= 1;
                            if self.text(self.hash_used).0 == 0 {
                                break;
                            }
                        }
                        self.set_next(p, self.hash_used as u16);
                        p = self.hash_used;
                    }
                    if io.pool_ptr + l > io.config.pool_size {
                        let n = (io.config.pool_size - io.init_pool_ptr) as i32;
                        return Err((StrNum::POOL_SIZE, n));
                    }
                    let d = io.cur_length();
                    while io.pool_ptr > io.str_start[io.str_ptr] {
                        io.pool_ptr -= 1;
                        io.str_pool[io.pool_ptr + l] = io.str_pool[io.pool_ptr];
                    }
                    for k in j..(j + l) {
                        let b = io.buffer[k];
                        io.append_char(b);
                    }
                    if io.str_ptr == io.config.max_strings {
                        let n = (io.config.max_strings - io.init_str_ptr) as i32;
                        return Err((StrNum::NUMBER_OF_STRINGS, n));
                    }
                    io.str_ptr += 1;
                    io.str_start[io.str_ptr] = io.pool_ptr;
                    let s = StrNum((io.str_ptr - 1) as u16);
                    self.set_text(p, s);
                    io.pool_ptr += d;
                    self.cs_count += 1;
                }
                return Ok(p as u16);
            }
            p = usize::from(self.next(p));
        }
    }

    /// §262: `print_cs(p)`.
    pub fn print_cs(&self, io: &mut IoState, eqtb: &Eqtb, p: i32) {
        let layout = &eqtb.layout;
        if p < layout.hash_base as i32 {
            if p >= layout.single_base as i32 {
                if p == layout.null_cs as i32 {
                    io.print_esc(pool_str("csname"));
                    io.print_esc(pool_str("endcsname"));
                    io.print_char(b' ');
                } else {
                    let c = (p - layout.single_base as i32) as u8;
                    io.print_esc(StrNum::from_ascii(c));
                    if eqtb.cat_code(c) == cmd::LETTER {
                        io.print_char(b' ');
                    }
                }
            } else if p < layout.active_base as i32 {
                io.print_esc(pool_str("IMPOSSIBLE."));
            } else {
                io.print(p - layout.active_base as i32);
            }
        } else if p >= layout.undefined_control_sequence as i32 {
            io.print_esc(pool_str("IMPOSSIBLE."));
        } else {
            let txt = self.text(p as usize);
            if usize::from(txt.0) >= io.str_ptr {
                io.print_esc(pool_str("NONEXISTENT."));
            } else {
                io.print_esc(txt);
                io.print_char(b' ');
            }
        }
    }

    /// §263: `sprint_cs(p)`.
    pub fn sprint_cs(&self, io: &mut IoState, layout: &EqtbLayout, p: u16) {
        let p_usz = usize::from(p);
        if p_usz < layout.hash_base {
            if p_usz < layout.single_base {
                io.print((p_usz - layout.active_base) as i32);
            } else if p_usz < layout.null_cs {
                io.print_esc(StrNum((p_usz - layout.single_base) as u16));
            } else {
                io.print_esc(pool_str("csname"));
                io.print_esc(pool_str("endcsname"));
            }
        } else {
            io.print_esc(self.text(p_usz));
        }
    }

    /// §264: `primitive(s, c, o)`. Returns `cur_val` (the `eqtb` location of the primitive).
    pub fn primitive(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        s: StrNum,
        c: u8,
        o: u16,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<u16, Termination> {
        let layout = eqtb.layout;
        let cur_val = if s.0 < 256 {
            s.0 + (layout.single_base as u16)
        } else {
            let k = io.str_start[usize::from(s.0)];
            let l = io.str_start[usize::from(s.0) + 1] - k;
            for j in 0..l {
                io.buffer[j] = io.str_pool[k + j];
            }
            let val = self.id_lookup(io, &layout, 0, l, hooks)?;
            io.flush_string();
            self.set_text(usize::from(val), s);
            val
        };
        let idx = usize::from(cur_val);
        eqtb.set_eq_level(idx, LEVEL_ONE);
        eqtb.set_eq_type(idx, c);
        eqtb.set_equiv(idx, o);
        Ok(cur_val)
    }

    /// §1336: `init_prim` registers all TeX primitives in exact source order (`§§226..1344`).
    pub fn init_prim(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        memory: &Memory,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        let layout = eqtb.layout;
        self.no_new_control_sequence = false;

        // §226: Glue parameters.
        for &(name, code) in &[
            ("lineskip", LINE_SKIP_CODE),
            ("baselineskip", BASELINE_SKIP_CODE),
            ("parskip", PAR_SKIP_CODE),
            ("abovedisplayskip", ABOVE_DISPLAY_SKIP_CODE),
            ("belowdisplayskip", BELOW_DISPLAY_SKIP_CODE),
            ("abovedisplayshortskip", ABOVE_DISPLAY_SHORT_SKIP_CODE),
            ("belowdisplayshortskip", BELOW_DISPLAY_SHORT_SKIP_CODE),
            ("leftskip", LEFT_SKIP_CODE),
            ("rightskip", RIGHT_SKIP_CODE),
            ("topskip", TOP_SKIP_CODE),
            ("splittopskip", SPLIT_TOP_SKIP_CODE),
            ("tabskip", TAB_SKIP_CODE),
            ("spaceskip", SPACE_SKIP_CODE),
            ("xspaceskip", XSPACE_SKIP_CODE),
            ("parfillskip", PAR_FILL_SKIP_CODE),
        ] {
            self.primitive(
                io,
                eqtb,
                pool_str(name),
                cmd::ASSIGN_GLUE,
                (layout.glue_base + code) as u16,
                hooks,
            )?;
        }
        for &(name, code) in &[
            ("thinmuskip", THIN_MU_SKIP_CODE),
            ("medmuskip", MED_MU_SKIP_CODE),
            ("thickmuskip", THICK_MU_SKIP_CODE),
        ] {
            self.primitive(
                io,
                eqtb,
                pool_str(name),
                cmd::ASSIGN_MU_GLUE,
                (layout.glue_base + code) as u16,
                hooks,
            )?;
        }

        // §230: Special token lists.
        for &(name, loc) in &[
            ("output", layout.output_routine_loc),
            ("everypar", layout.every_par_loc),
            ("everymath", layout.every_math_loc),
            ("everydisplay", layout.every_display_loc),
            ("everyhbox", layout.every_hbox_loc),
            ("everyvbox", layout.every_vbox_loc),
            ("everyjob", layout.every_job_loc),
            ("everycr", layout.every_cr_loc),
            ("errhelp", layout.err_help_loc),
        ] {
            self.primitive(
                io,
                eqtb,
                pool_str(name),
                cmd::ASSIGN_TOKS,
                loc as u16,
                hooks,
            )?;
        }

        // §238: Integer parameters.
        for &(name, code) in &[
            ("pretolerance", PRETOLERANCE_CODE),
            ("tolerance", TOLERANCE_CODE),
            ("linepenalty", LINE_PENALTY_CODE),
            ("hyphenpenalty", HYPHEN_PENALTY_CODE),
            ("exhyphenpenalty", EX_HYPHEN_PENALTY_CODE),
            ("clubpenalty", CLUB_PENALTY_CODE),
            ("widowpenalty", WIDOW_PENALTY_CODE),
            ("displaywidowpenalty", DISPLAY_WIDOW_PENALTY_CODE),
            ("brokenpenalty", BROKEN_PENALTY_CODE),
            ("binoppenalty", BIN_OP_PENALTY_CODE),
            ("relpenalty", REL_PENALTY_CODE),
            ("predisplaypenalty", PRE_DISPLAY_PENALTY_CODE),
            ("postdisplaypenalty", POST_DISPLAY_PENALTY_CODE),
            ("interlinepenalty", INTER_LINE_PENALTY_CODE),
            ("doublehyphendemerits", DOUBLE_HYPHEN_DEMERITS_CODE),
            ("finalhyphendemerits", FINAL_HYPHEN_DEMERITS_CODE),
            ("adjdemerits", ADJ_DEMERITS_CODE),
            ("mag", MAG_CODE),
            ("delimiterfactor", DELIMITER_FACTOR_CODE),
            ("looseness", LOOSENESS_CODE),
            ("time", TIME_CODE),
            ("day", DAY_CODE),
            ("month", MONTH_CODE),
            ("year", YEAR_CODE),
            ("showboxbreadth", SHOW_BOX_BREADTH_CODE),
            ("showboxdepth", SHOW_BOX_DEPTH_CODE),
            ("hbadness", HBADNESS_CODE),
            ("vbadness", VBADNESS_CODE),
            ("pausing", PAUSING_CODE),
            ("tracingonline", TRACING_ONLINE_CODE),
            ("tracingmacros", TRACING_MACROS_CODE),
            ("tracingstats", TRACING_STATS_CODE),
            ("tracingparagraphs", TRACING_PARAGRAPHS_CODE),
            ("tracingpages", TRACING_PAGES_CODE),
            ("tracingoutput", TRACING_OUTPUT_CODE),
            ("tracinglostchars", TRACING_LOST_CHARS_CODE),
            ("tracingcommands", TRACING_COMMANDS_CODE),
            ("tracingrestores", TRACING_RESTORES_CODE),
            ("uchyph", UC_HYPH_CODE),
            ("outputpenalty", OUTPUT_PENALTY_CODE),
            ("maxdeadcycles", MAX_DEAD_CYCLES_CODE),
            ("hangafter", HANG_AFTER_CODE),
            ("floatingpenalty", FLOATING_PENALTY_CODE),
            ("globaldefs", GLOBAL_DEFS_CODE),
            ("fam", CUR_FAM_CODE),
            ("escapechar", ESCAPE_CHAR_CODE),
            ("defaulthyphenchar", DEFAULT_HYPHEN_CHAR_CODE),
            ("defaultskewchar", DEFAULT_SKEW_CHAR_CODE),
            ("endlinechar", END_LINE_CHAR_CODE),
            ("newlinechar", NEW_LINE_CHAR_CODE),
            ("language", LANGUAGE_CODE),
            ("lefthyphenmin", LEFT_HYPHEN_MIN_CODE),
            ("righthyphenmin", RIGHT_HYPHEN_MIN_CODE),
            ("holdinginserts", HOLDING_INSERTS_CODE),
            ("errorcontextlines", ERROR_CONTEXT_LINES_CODE),
        ] {
            self.primitive(
                io,
                eqtb,
                pool_str(name),
                cmd::ASSIGN_INT,
                (layout.int_base + code) as u16,
                hooks,
            )?;
        }

        // §248: Dimension parameters.
        for &(name, code) in &[
            ("parindent", PAR_INDENT_CODE),
            ("mathsurround", MATH_SURROUND_CODE),
            ("lineskiplimit", LINE_SKIP_LIMIT_CODE),
            ("hsize", HSIZE_CODE),
            ("vsize", VSIZE_CODE),
            ("maxdepth", MAX_DEPTH_CODE),
            ("splitmaxdepth", SPLIT_MAX_DEPTH_CODE),
            ("boxmaxdepth", BOX_MAX_DEPTH_CODE),
            ("hfuzz", HFUZZ_CODE),
            ("vfuzz", VFUZZ_CODE),
            ("delimitershortfall", DELIMITER_SHORTFALL_CODE),
            ("nulldelimiterspace", NULL_DELIMITER_SPACE_CODE),
            ("scriptspace", SCRIPT_SPACE_CODE),
            ("predisplaysize", PRE_DISPLAY_SIZE_CODE),
            ("displaywidth", DISPLAY_WIDTH_CODE),
            ("displayindent", DISPLAY_INDENT_CODE),
            ("overfullrule", OVERFULL_RULE_CODE),
            ("hangindent", HANG_INDENT_CODE),
            ("hoffset", H_OFFSET_CODE),
            ("voffset", V_OFFSET_CODE),
            ("emergencystretch", EMERGENCY_STRETCH_CODE),
        ] {
            self.primitive(
                io,
                eqtb,
                pool_str(name),
                cmd::ASSIGN_DIMEN,
                (layout.dimen_base + code) as u16,
                hooks,
            )?;
        }

        // §265: General primitives.
        self.primitive(io, eqtb, pool_str(" "), cmd::EX_SPACE, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("/"), cmd::ITAL_CORR, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("accent"), cmd::ACCENT, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("advance"), cmd::ADVANCE, 0, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("afterassignment"),
            cmd::AFTER_ASSIGNMENT,
            0,
            hooks,
        )?;
        self.primitive(io, eqtb, pool_str("aftergroup"), cmd::AFTER_GROUP, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("begingroup"), cmd::BEGIN_GROUP, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("char"), cmd::CHAR_NUM, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("csname"), cmd::CS_NAME, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("delimiter"), cmd::DELIM_NUM, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("divide"), cmd::DIVIDE, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("endcsname"), cmd::END_CS_NAME, 0, hooks)?;
        let cur_val = self.primitive(io, eqtb, pool_str("endgroup"), cmd::END_GROUP, 0, hooks)?;
        self.set_text(layout.frozen_end_group, pool_str("endgroup"));
        eqtb.words[layout.frozen_end_group] = eqtb.words[usize::from(cur_val)];
        self.primitive(
            io,
            eqtb,
            pool_str("expandafter"),
            cmd::EXPAND_AFTER,
            0,
            hooks,
        )?;
        self.primitive(io, eqtb, pool_str("font"), cmd::DEF_FONT, 0, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("fontdimen"),
            cmd::ASSIGN_FONT_DIMEN,
            0,
            hooks,
        )?;
        self.primitive(io, eqtb, pool_str("halign"), cmd::HALIGN, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("hrule"), cmd::HRULE, 0, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("ignorespaces"),
            cmd::IGNORE_SPACES,
            0,
            hooks,
        )?;
        self.primitive(io, eqtb, pool_str("insert"), cmd::INSERT, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("mark"), cmd::MARK, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("mathaccent"), cmd::MATH_ACCENT, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("mathchar"), cmd::MATH_CHAR_NUM, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("mathchoice"), cmd::MATH_CHOICE, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("multiply"), cmd::MULTIPLY, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("noalign"), cmd::NO_ALIGN, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("noboundary"), cmd::NO_BOUNDARY, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("noexpand"), cmd::NO_EXPAND, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("nonscript"), cmd::NON_SCRIPT, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("omit"), cmd::OMIT, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("parshape"), cmd::SET_SHAPE, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("penalty"), cmd::BREAK_PENALTY, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("prevgraf"), cmd::SET_PREV_GRAF, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("radical"), cmd::RADICAL, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("read"), cmd::READ_TO_CS, 0, hooks)?;
        let cur_val = self.primitive(io, eqtb, pool_str("relax"), cmd::RELAX, 256, hooks)?;
        self.set_text(layout.frozen_relax, pool_str("relax"));
        eqtb.words[layout.frozen_relax] = eqtb.words[usize::from(cur_val)];
        self.primitive(io, eqtb, pool_str("setbox"), cmd::SET_BOX, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("the"), cmd::THE, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("toks"), cmd::TOKS_REGISTER, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("vadjust"), cmd::VADJUST, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("valign"), cmd::VALIGN, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("vcenter"), cmd::VCENTER, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("vrule"), cmd::VRULE, 0, hooks)?;

        // §334: \par
        let cur_val = self.primitive(io, eqtb, pool_str("par"), cmd::PAR_END, 256, hooks)?;
        self.par_loc = cur_val;
        self.par_token = Token::from_cs(cur_val);

        // §376: \input, \endinput
        self.primitive(io, eqtb, pool_str("input"), cmd::INPUT, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("endinput"), cmd::INPUT, 1, hooks)?;

        // §384: Marks
        self.primitive(
            io,
            eqtb,
            pool_str("topmark"),
            cmd::TOP_BOT_MARK,
            TOP_MARK_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("firstmark"),
            cmd::TOP_BOT_MARK,
            FIRST_MARK_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("botmark"),
            cmd::TOP_BOT_MARK,
            BOT_MARK_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("splitfirstmark"),
            cmd::TOP_BOT_MARK,
            SPLIT_FIRST_MARK_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("splitbotmark"),
            cmd::TOP_BOT_MARK,
            SPLIT_BOT_MARK_CODE,
            hooks,
        )?;

        // §411: Registers
        self.primitive(io, eqtb, pool_str("count"), cmd::REGISTER, INT_VAL, hooks)?;
        self.primitive(io, eqtb, pool_str("dimen"), cmd::REGISTER, DIMEN_VAL, hooks)?;
        self.primitive(io, eqtb, pool_str("skip"), cmd::REGISTER, GLUE_VAL, hooks)?;
        self.primitive(io, eqtb, pool_str("muskip"), cmd::REGISTER, MU_VAL, hooks)?;

        // §416: Internal quantities
        self.primitive(
            io,
            eqtb,
            pool_str("spacefactor"),
            cmd::SET_AUX,
            HMODE as u16,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("prevdepth"),
            cmd::SET_AUX,
            VMODE as u16,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("deadcycles"),
            cmd::SET_PAGE_INT,
            0,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("insertpenalties"),
            cmd::SET_PAGE_INT,
            1,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("wd"),
            cmd::SET_BOX_DIMEN,
            WIDTH_OFFSET,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("ht"),
            cmd::SET_BOX_DIMEN,
            HEIGHT_OFFSET,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("dp"),
            cmd::SET_BOX_DIMEN,
            DEPTH_OFFSET,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("lastpenalty"),
            cmd::LAST_ITEM,
            INT_VAL,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("lastkern"),
            cmd::LAST_ITEM,
            DIMEN_VAL,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("lastskip"),
            cmd::LAST_ITEM,
            GLUE_VAL,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("inputlineno"),
            cmd::LAST_ITEM,
            INPUT_LINE_NO_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("badness"),
            cmd::LAST_ITEM,
            BADNESS_CODE,
            hooks,
        )?;

        // §468: Convert primitives
        self.primitive(
            io,
            eqtb,
            pool_str("number"),
            cmd::CONVERT,
            NUMBER_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("romannumeral"),
            cmd::CONVERT,
            ROMAN_NUMERAL_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("string"),
            cmd::CONVERT,
            STRING_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("meaning"),
            cmd::CONVERT,
            MEANING_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("fontname"),
            cmd::CONVERT,
            FONT_NAME_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("jobname"),
            cmd::CONVERT,
            JOB_NAME_CODE,
            hooks,
        )?;

        // §487: Conditionals
        for &(name, code) in &[
            ("if", IF_CHAR_CODE),
            ("ifcat", IF_CAT_CODE),
            ("ifnum", IF_INT_CODE),
            ("ifdim", IF_DIM_CODE),
            ("ifodd", IF_ODD_CODE),
            ("ifvmode", IF_VMODE_CODE),
            ("ifhmode", IF_HMODE_CODE),
            ("ifmmode", IF_MMODE_CODE),
            ("ifinner", IF_INNER_CODE),
            ("ifvoid", IF_VOID_CODE),
            ("ifhbox", IF_HBOX_CODE),
            ("ifvbox", IF_VBOX_CODE),
            ("ifx", IFX_CODE),
            ("ifeof", IF_EOF_CODE),
            ("iftrue", IF_TRUE_CODE),
            ("iffalse", IF_FALSE_CODE),
            ("ifcase", IF_CASE_CODE),
        ] {
            self.primitive(io, eqtb, pool_str(name), cmd::IF_TEST, code, hooks)?;
        }

        // §491: \fi, \or, \else
        let cur_val = self.primitive(io, eqtb, pool_str("fi"), cmd::FI_OR_ELSE, FI_CODE, hooks)?;
        self.set_text(layout.frozen_fi, pool_str("fi"));
        eqtb.words[layout.frozen_fi] = eqtb.words[usize::from(cur_val)];
        self.primitive(io, eqtb, pool_str("or"), cmd::FI_OR_ELSE, OR_CODE, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("else"),
            cmd::FI_OR_ELSE,
            ELSE_CODE,
            hooks,
        )?;

        // §553: \nullfont
        let cur_val = self.primitive(
            io,
            eqtb,
            pool_str("nullfont"),
            cmd::SET_FONT,
            NULL_FONT,
            hooks,
        )?;
        self.set_text(layout.frozen_null_font, pool_str("nullfont"));
        eqtb.words[layout.frozen_null_font] = eqtb.words[usize::from(cur_val)];

        // §780: \span, \cr, \crcr, \endtemplate
        self.primitive(io, eqtb, pool_str("span"), cmd::TAB_MARK, SPAN_CODE, hooks)?;
        let cur_val = self.primitive(io, eqtb, pool_str("cr"), cmd::CAR_RET, CR_CODE, hooks)?;
        self.set_text(layout.frozen_cr, pool_str("cr"));
        eqtb.words[layout.frozen_cr] = eqtb.words[usize::from(cur_val)];
        self.primitive(io, eqtb, pool_str("crcr"), cmd::CAR_RET, CR_CR_CODE, hooks)?;
        self.set_text(layout.frozen_end_template, pool_str("endtemplate"));
        self.set_text(layout.frozen_endv, pool_str("endtemplate"));
        eqtb.words[layout.frozen_endv] =
            EqWord::from_hh(cmd::ENDV, LEVEL_ONE, memory.null_list().address());
        eqtb.words[layout.frozen_end_template] = eqtb.words[layout.frozen_endv];
        eqtb.set_eq_type(layout.frozen_end_template, cmd::END_TEMPLATE);

        // §983: Page dimensions
        for &(name, code) in &[
            ("pagegoal", 0),
            ("pagetotal", 1),
            ("pagestretch", 2),
            ("pagefilstretch", 3),
            ("pagefillstretch", 4),
            ("pagefilllstretch", 5),
            ("pageshrink", 6),
            ("pagedepth", 7),
        ] {
            self.primitive(io, eqtb, pool_str(name), cmd::SET_PAGE_DIMEN, code, hooks)?;
        }

        // §1052: \end, \dump
        self.primitive(io, eqtb, pool_str("end"), cmd::STOP, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("dump"), cmd::STOP, 1, hooks)?;

        // §1058: Skips and kerns
        self.primitive(io, eqtb, pool_str("hskip"), cmd::HSKIP, SKIP_CODE, hooks)?;
        self.primitive(io, eqtb, pool_str("hfil"), cmd::HSKIP, FIL_CODE, hooks)?;
        self.primitive(io, eqtb, pool_str("hfill"), cmd::HSKIP, FILL_CODE, hooks)?;
        self.primitive(io, eqtb, pool_str("hss"), cmd::HSKIP, SS_CODE, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("hfilneg"),
            cmd::HSKIP,
            FIL_NEG_CODE,
            hooks,
        )?;
        self.primitive(io, eqtb, pool_str("vskip"), cmd::VSKIP, SKIP_CODE, hooks)?;
        self.primitive(io, eqtb, pool_str("vfil"), cmd::VSKIP, FIL_CODE, hooks)?;
        self.primitive(io, eqtb, pool_str("vfill"), cmd::VSKIP, FILL_CODE, hooks)?;
        self.primitive(io, eqtb, pool_str("vss"), cmd::VSKIP, SS_CODE, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("vfilneg"),
            cmd::VSKIP,
            FIL_NEG_CODE,
            hooks,
        )?;
        self.primitive(io, eqtb, pool_str("mskip"), cmd::MSKIP, MSKIP_CODE, hooks)?;
        self.primitive(io, eqtb, pool_str("kern"), cmd::KERN, 1, hooks)?;
        self.primitive(io, eqtb, pool_str("mkern"), cmd::MKERN, 99, hooks)?;

        // §1071: Boxes and leaders
        self.primitive(io, eqtb, pool_str("moveleft"), cmd::HMOVE, 1, hooks)?;
        self.primitive(io, eqtb, pool_str("moveright"), cmd::HMOVE, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("raise"), cmd::VMOVE, 1, hooks)?;
        self.primitive(io, eqtb, pool_str("lower"), cmd::VMOVE, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("box"), cmd::MAKE_BOX, BOX_CODE, hooks)?;
        self.primitive(io, eqtb, pool_str("copy"), cmd::MAKE_BOX, COPY_CODE, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("lastbox"),
            cmd::MAKE_BOX,
            LAST_BOX_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("vsplit"),
            cmd::MAKE_BOX,
            VSPLIT_CODE,
            hooks,
        )?;
        self.primitive(io, eqtb, pool_str("vtop"), cmd::MAKE_BOX, VTOP_CODE, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("vbox"),
            cmd::MAKE_BOX,
            VTOP_CODE + (VMODE as u16),
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("hbox"),
            cmd::MAKE_BOX,
            VTOP_CODE + (HMODE as u16),
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("shipout"),
            cmd::LEADER_SHIP,
            A_LEADERS - 1,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("leaders"),
            cmd::LEADER_SHIP,
            A_LEADERS,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("cleaders"),
            cmd::LEADER_SHIP,
            C_LEADERS,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("xleaders"),
            cmd::LEADER_SHIP,
            X_LEADERS,
            hooks,
        )?;

        // §1088: \indent, \noindent
        self.primitive(io, eqtb, pool_str("indent"), cmd::START_PAR, 1, hooks)?;
        self.primitive(io, eqtb, pool_str("noindent"), cmd::START_PAR, 0, hooks)?;

        // §1107: \unpenalty, \unkern, \unskip, \unhbox, \unhcopy, \unvbox, \unvcopy
        self.primitive(
            io,
            eqtb,
            pool_str("unpenalty"),
            cmd::REMOVE_ITEM,
            PENALTY_NODE_TYPE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("unkern"),
            cmd::REMOVE_ITEM,
            KERN_NODE_TYPE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("unskip"),
            cmd::REMOVE_ITEM,
            GLUE_NODE_TYPE,
            hooks,
        )?;
        self.primitive(io, eqtb, pool_str("unhbox"), cmd::UN_HBOX, BOX_CODE, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("unhcopy"),
            cmd::UN_HBOX,
            COPY_CODE,
            hooks,
        )?;
        self.primitive(io, eqtb, pool_str("unvbox"), cmd::UN_VBOX, BOX_CODE, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("unvcopy"),
            cmd::UN_VBOX,
            COPY_CODE,
            hooks,
        )?;

        // §1114: \-, \discretionary
        self.primitive(io, eqtb, pool_str("-"), cmd::DISCRETIONARY, 1, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("discretionary"),
            cmd::DISCRETIONARY,
            0,
            hooks,
        )?;

        // §1141: \eqno, \leqno
        self.primitive(io, eqtb, pool_str("eqno"), cmd::EQ_NO, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("leqno"), cmd::EQ_NO, 1, hooks)?;

        // §1156: Math components and limit switches
        for &(name, code) in &[
            ("mathord", ORD_NOAD),
            ("mathop", OP_NOAD),
            ("mathbin", BIN_NOAD),
            ("mathrel", REL_NOAD),
            ("mathopen", OPEN_NOAD),
            ("mathclose", CLOSE_NOAD),
            ("mathpunct", PUNCT_NOAD),
            ("mathinner", INNER_NOAD),
            ("underline", UNDER_NOAD),
            ("overline", OVER_NOAD),
        ] {
            self.primitive(io, eqtb, pool_str(name), cmd::MATH_COMP, code, hooks)?;
        }
        self.primitive(
            io,
            eqtb,
            pool_str("displaylimits"),
            cmd::LIMIT_SWITCH,
            0,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("limits"),
            cmd::LIMIT_SWITCH,
            LIMITS,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("nolimits"),
            cmd::LIMIT_SWITCH,
            NO_LIMITS,
            hooks,
        )?;

        // §1169: Math styles
        self.primitive(
            io,
            eqtb,
            pool_str("displaystyle"),
            cmd::MATH_STYLE,
            DISPLAY_STYLE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("textstyle"),
            cmd::MATH_STYLE,
            TEXT_STYLE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("scriptstyle"),
            cmd::MATH_STYLE,
            SCRIPT_STYLE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("scriptscriptstyle"),
            cmd::MATH_STYLE,
            SCRIPT_SCRIPT_STYLE,
            hooks,
        )?;

        // §1178: Generalized fractions
        self.primitive(io, eqtb, pool_str("above"), cmd::ABOVE, ABOVE_CODE, hooks)?;
        self.primitive(io, eqtb, pool_str("over"), cmd::ABOVE, OVER_CODE, hooks)?;
        self.primitive(io, eqtb, pool_str("atop"), cmd::ABOVE, ATOP_CODE, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("abovewithdelims"),
            cmd::ABOVE,
            DELIMITED_CODE + ABOVE_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("overwithdelims"),
            cmd::ABOVE,
            DELIMITED_CODE + OVER_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("atopwithdelims"),
            cmd::ABOVE,
            DELIMITED_CODE + ATOP_CODE,
            hooks,
        )?;

        // §1188: \left, \right
        self.primitive(
            io,
            eqtb,
            pool_str("left"),
            cmd::LEFT_RIGHT,
            LEFT_NOAD,
            hooks,
        )?;
        let cur_val = self.primitive(
            io,
            eqtb,
            pool_str("right"),
            cmd::LEFT_RIGHT,
            RIGHT_NOAD,
            hooks,
        )?;
        self.set_text(layout.frozen_right, pool_str("right"));
        eqtb.words[layout.frozen_right] = eqtb.words[usize::from(cur_val)];

        // §1208: Prefixes and macro defs
        self.primitive(io, eqtb, pool_str("long"), cmd::PREFIX, 1, hooks)?;
        self.primitive(io, eqtb, pool_str("outer"), cmd::PREFIX, 2, hooks)?;
        self.primitive(io, eqtb, pool_str("global"), cmd::PREFIX, 4, hooks)?;
        self.primitive(io, eqtb, pool_str("def"), cmd::DEF, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("gdef"), cmd::DEF, 1, hooks)?;
        self.primitive(io, eqtb, pool_str("edef"), cmd::DEF, 2, hooks)?;
        self.primitive(io, eqtb, pool_str("xdef"), cmd::DEF, 3, hooks)?;

        // §1219: \let, \futurelet
        self.primitive(io, eqtb, pool_str("let"), cmd::LET, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("futurelet"), cmd::LET, 1, hooks)?;

        // §1222: Shorthand definitions
        for &(name, code) in &[
            ("chardef", CHAR_DEF_CODE),
            ("mathchardef", MATH_CHAR_DEF_CODE),
            ("countdef", COUNT_DEF_CODE),
            ("dimendef", DIMEN_DEF_CODE),
            ("skipdef", SKIP_DEF_CODE),
            ("muskipdef", MU_SKIP_DEF_CODE),
            ("toksdef", TOKS_DEF_CODE),
        ] {
            self.primitive(io, eqtb, pool_str(name), cmd::SHORTHAND_DEF, code, hooks)?;
        }

        // §1230: Code tables and font families
        for &(name, base) in &[
            ("catcode", layout.cat_code_base),
            ("mathcode", layout.math_code_base),
            ("lccode", layout.lc_code_base),
            ("uccode", layout.uc_code_base),
            ("sfcode", layout.sf_code_base),
            ("delcode", layout.del_code_base),
        ] {
            self.primitive(io, eqtb, pool_str(name), cmd::DEF_CODE, base as u16, hooks)?;
        }
        self.primitive(
            io,
            eqtb,
            pool_str("textfont"),
            cmd::DEF_FAMILY,
            layout.math_font_base as u16,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("scriptfont"),
            cmd::DEF_FAMILY,
            (layout.math_font_base as u16) + SCRIPT_SIZE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("scriptscriptfont"),
            cmd::DEF_FAMILY,
            (layout.math_font_base as u16) + SCRIPT_SCRIPT_SIZE,
            hooks,
        )?;

        // §1250: Hyphenation data
        self.primitive(io, eqtb, pool_str("hyphenation"), cmd::HYPH_DATA, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("patterns"), cmd::HYPH_DATA, 1, hooks)?;

        // §1254: Font integers
        self.primitive(
            io,
            eqtb,
            pool_str("hyphenchar"),
            cmd::ASSIGN_FONT_INT,
            0,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("skewchar"),
            cmd::ASSIGN_FONT_INT,
            1,
            hooks,
        )?;

        // §1262: Interaction modes
        self.primitive(
            io,
            eqtb,
            pool_str("batchmode"),
            cmd::SET_INTERACTION,
            0,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("nonstopmode"),
            cmd::SET_INTERACTION,
            1,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("scrollmode"),
            cmd::SET_INTERACTION,
            2,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("errorstopmode"),
            cmd::SET_INTERACTION,
            3,
            hooks,
        )?;

        // §1272: \openin, \closein
        self.primitive(io, eqtb, pool_str("openin"), cmd::IN_STREAM, 1, hooks)?;
        self.primitive(io, eqtb, pool_str("closein"), cmd::IN_STREAM, 0, hooks)?;

        // §1277: \message, \errmessage
        self.primitive(io, eqtb, pool_str("message"), cmd::MESSAGE, 0, hooks)?;
        self.primitive(io, eqtb, pool_str("errmessage"), cmd::MESSAGE, 1, hooks)?;

        // §1286: \lowercase, \uppercase
        self.primitive(
            io,
            eqtb,
            pool_str("lowercase"),
            cmd::CASE_SHIFT,
            layout.lc_code_base as u16,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("uppercase"),
            cmd::CASE_SHIFT,
            layout.uc_code_base as u16,
            hooks,
        )?;

        // §1291: \show, \showbox, \showthe, \showlists
        self.primitive(io, eqtb, pool_str("show"), cmd::XRAY, SHOW_CODE, hooks)?;
        self.primitive(
            io,
            eqtb,
            pool_str("showbox"),
            cmd::XRAY,
            SHOW_BOX_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("showthe"),
            cmd::XRAY,
            SHOW_THE_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("showlists"),
            cmd::XRAY,
            SHOW_LISTS_CODE,
            hooks,
        )?;

        // §1344: Extensions
        self.primitive(
            io,
            eqtb,
            pool_str("openout"),
            cmd::EXTENSION,
            OPEN_NODE,
            hooks,
        )?;
        let write_loc = self.primitive(
            io,
            eqtb,
            pool_str("write"),
            cmd::EXTENSION,
            WRITE_NODE,
            hooks,
        )?;
        self.write_loc = write_loc;
        self.primitive(
            io,
            eqtb,
            pool_str("closeout"),
            cmd::EXTENSION,
            CLOSE_NODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("special"),
            cmd::EXTENSION,
            SPECIAL_NODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("immediate"),
            cmd::EXTENSION,
            IMMEDIATE_CODE,
            hooks,
        )?;
        self.primitive(
            io,
            eqtb,
            pool_str("setlanguage"),
            cmd::EXTENSION,
            SET_LANGUAGE_CODE,
            hooks,
        )?;

        self.no_new_control_sequence = true;
        Ok(())
    }
}

/// §268: Save-stack entry types (`0..=3`).
pub const RESTORE_OLD_VALUE: u8 = 0;
pub const RESTORE_ZERO: u8 = 1;
pub const INSERT_TOKEN: u8 = 2;
pub const LEVEL_BOUNDARY: u8 = 3;

/// §269: Group codes (`0..=16`).
pub mod group {
    pub const BOTTOM_LEVEL: u8 = 0;
    pub const SIMPLE_GROUP: u8 = 1;
    pub const HBOX_GROUP: u8 = 2;
    pub const ADJUSTED_HBOX_GROUP: u8 = 3;
    pub const VBOX_GROUP: u8 = 4;
    pub const VTOP_GROUP: u8 = 5;
    pub const ALIGN_GROUP: u8 = 6;
    pub const NO_ALIGN_GROUP: u8 = 7;
    pub const OUTPUT_GROUP: u8 = 8;
    pub const MATH_GROUP: u8 = 9;
    pub const DISC_GROUP: u8 = 10;
    pub const INSERT_GROUP: u8 = 11;
    pub const VCENTER_GROUP: u8 = 12;
    pub const MATH_CHOICE_GROUP: u8 = 13;
    pub const SEMI_SIMPLE_GROUP: u8 = 14;
    pub const MATH_SHIFT_GROUP: u8 = 15;
    pub const MATH_LEFT_GROUP: u8 = 16;
    pub const MAX_GROUP_CODE: u8 = 16;
}

/// §§268–288: The save stack and grouping state.
#[derive(Clone, Debug)]
pub struct SaveStack {
    pub stack: Vec<EqWord>,
    pub save_ptr: usize,
    pub max_save_stack: usize,
    pub save_size: usize,
    pub cur_level: u8,
    pub cur_group: u8,
    pub cur_boundary: usize,
    pub mag_set: i32,
}

impl SaveStack {
    /// §§272, 287: Initialize `save_stack` at `level_one` and `bottom_level`.
    pub fn new(save_size: usize) -> Self {
        Self {
            stack: vec![EqWord::default(); save_size + 1],
            save_ptr: 0,
            max_save_stack: 0,
            save_size,
            cur_level: LEVEL_ONE,
            cur_group: group::BOTTOM_LEVEL,
            cur_boundary: 0,
            mag_set: 0,
        }
    }

    pub fn saved(&self, offset: isize) -> i32 {
        let idx = (self.save_ptr as isize + offset) as usize;
        self.stack[idx].int()
    }

    pub fn set_saved(&mut self, offset: isize, val: i32) {
        let idx = (self.save_ptr as isize + offset) as usize;
        self.stack[idx].set_int(val);
    }

    /// §273: `check_full_save_stack`.
    pub fn check_full_save_stack(
        &mut self,
        io: &mut IoState,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        if self.save_ptr > self.max_save_stack {
            self.max_save_stack = self.save_ptr;
            if self.max_save_stack + 6 > self.save_size {
                return Err(io.overflow_with_hooks(
                    pool_str("save size"),
                    self.save_size as i32,
                    hooks,
                ));
            }
        }
        Ok(())
    }

    /// §274: `new_save_level(c)`.
    pub fn new_save_level(
        &mut self,
        io: &mut IoState,
        c: u8,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        self.check_full_save_stack(io, hooks)?;
        self.stack[self.save_ptr] =
            EqWord::from_hh(LEVEL_BOUNDARY, self.cur_group, self.cur_boundary as u16);
        if self.cur_level == u8::MAX {
            return Err(io.overflow_with_hooks(
                pool_str("grouping levels"),
                i32::from(u8::MAX),
                hooks,
            ));
        }
        self.cur_boundary = self.save_ptr;
        self.cur_level += 1;
        self.save_ptr += 1;
        self.cur_group = c;
        Ok(())
    }

    /// §275: `eq_destroy(w)`.
    pub fn eq_destroy(memory: &mut Memory, w: EqWord) -> Result<(), MemoryError> {
        let q = MemPtr::from_address(w.rh());
        match w.b0() {
            cmd::CALL | cmd::LONG_CALL | cmd::OUTER_CALL | cmd::LONG_OUTER_CALL => {
                if q != MemPtr::NULL {
                    memory.delete_token_ref(q)?;
                }
            }
            cmd::GLUE_REF => {
                if q != MemPtr::NULL {
                    memory.delete_glue_ref(q)?;
                }
            }
            cmd::SHAPE_REF => {
                if q != MemPtr::NULL {
                    memory.free_par_shape(q)?;
                }
            }
            cmd::BOX_REF => {
                // Box node destruction connects in M5 when box nodes are allocated.
            }
            _ => {}
        }
        Ok(())
    }

    /// §276: `eq_save(p, l)`.
    pub fn eq_save(
        &mut self,
        io: &mut IoState,
        eqtb: &Eqtb,
        p: usize,
        l: u8,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        self.check_full_save_stack(io, hooks)?;
        if l == LEVEL_ZERO {
            self.stack[self.save_ptr] = EqWord::from_hh(RESTORE_ZERO, LEVEL_ZERO, p as u16);
            self.save_ptr += 1;
        } else {
            self.stack[self.save_ptr] = eqtb.words[p];
            self.save_ptr += 1;
            self.stack[self.save_ptr] = EqWord::from_hh(RESTORE_OLD_VALUE, l, p as u16);
            self.save_ptr += 1;
        }
        Ok(())
    }

    /// §277: `eq_define(p, t, e)` for regions 1–4 of `eqtb`.
    #[allow(clippy::too_many_arguments)]
    pub fn eq_define(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        memory: &mut Memory,
        p: usize,
        t: u8,
        e: u16,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        if eqtb.eq_level(p) == self.cur_level {
            let _ = Self::eq_destroy(memory, eqtb.words[p]);
        } else if self.cur_level > LEVEL_ONE {
            self.eq_save(io, eqtb, p, eqtb.eq_level(p), hooks)?;
        }
        eqtb.words[p] = EqWord::from_hh(t, self.cur_level, e);
        if p == eqtb.layout.err_help_loc {
            eqtb.sync_io_params(io);
        }
        Ok(())
    }

    /// §278: `eq_word_define(p, w)` for regions 5–6 of `eqtb`.
    pub fn eq_word_define(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        p: usize,
        w: i32,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        if eqtb.eq_level(p) != self.cur_level {
            self.eq_save(io, eqtb, p, eqtb.eq_level(p), hooks)?;
            eqtb.set_eq_level(p, self.cur_level);
        }
        eqtb.set_int(p, w);
        eqtb.sync_io_params(io);
        Ok(())
    }

    /// §279: `geq_define(p, t, e)` global definition for regions 1–4 of `eqtb`.
    pub fn geq_define(
        io: &mut IoState,
        eqtb: &mut Eqtb,
        memory: &mut Memory,
        p: usize,
        t: u8,
        e: u16,
    ) {
        let _ = Self::eq_destroy(memory, eqtb.words[p]);
        eqtb.words[p] = EqWord::from_hh(t, LEVEL_ONE, e);
        if p == eqtb.layout.err_help_loc {
            eqtb.sync_io_params(io);
        }
    }

    /// §279: `geq_word_define(p, w)` global definition for regions 5–6 of `eqtb`.
    pub fn geq_word_define(io: &mut IoState, eqtb: &mut Eqtb, p: usize, w: i32) {
        eqtb.set_int(p, w);
        eqtb.set_eq_level(p, LEVEL_ONE);
        eqtb.sync_io_params(io);
    }

    /// §280: `save_for_after(t)`.
    pub fn save_for_after(
        &mut self,
        io: &mut IoState,
        t: Token,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        if self.cur_level > LEVEL_ONE {
            self.check_full_save_stack(io, hooks)?;
            self.stack[self.save_ptr] = EqWord::from_hh(INSERT_TOKEN, LEVEL_ZERO, t.raw());
            self.save_ptr += 1;
        }
        Ok(())
    }

    /// §§281–284: `unsave`. Pops the top level off `save_stack`, restoring `eqtb` and returning
    /// any `\aftergroup` tokens (`insert_token`) in the exact order they are popped so the caller
    /// can insert them via `back_input`.
    pub fn unsave(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &HashTable,
        memory: &mut Memory,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<Vec<Token>, Termination> {
        if self.cur_level <= LEVEL_ONE {
            return Err(io.confusion_with_hooks(pool_str("curlevel"), hooks));
        }
        self.cur_level -= 1;
        let mut after_tokens = Vec::new();
        loop {
            self.save_ptr -= 1;
            let top = self.stack[self.save_ptr];
            let save_type = top.b0();
            if save_type == LEVEL_BOUNDARY {
                self.cur_group = top.b1();
                self.cur_boundary = usize::from(top.rh());
                break;
            }
            let p = usize::from(top.rh());
            if save_type == INSERT_TOKEN {
                after_tokens.push(Token(top.rh()));
            } else {
                let mut l = LEVEL_ZERO;
                if save_type == RESTORE_OLD_VALUE {
                    l = top.b1();
                    self.save_ptr -= 1;
                } else {
                    self.stack[self.save_ptr] = eqtb.words[eqtb.layout.undefined_control_sequence];
                }
                let saved_word = self.stack[self.save_ptr];
                let tracing_restores = eqtb.int_par(TRACING_RESTORES_CODE) > 0;
                // §283: Store save_stack[save_ptr] in eqtb[p], unless eqtb[p] holds a global value.
                if p < eqtb.layout.int_base {
                    if eqtb.eq_level(p) == LEVEL_ONE {
                        let _ = Self::eq_destroy(memory, saved_word);
                        if tracing_restores {
                            Self::restore_trace(io, eqtb, hash, memory, p, pool_str("retaining"));
                        }
                    } else {
                        let _ = Self::eq_destroy(memory, eqtb.words[p]);
                        eqtb.words[p] = saved_word;
                        if tracing_restores {
                            Self::restore_trace(io, eqtb, hash, memory, p, pool_str("restoring"));
                        }
                    }
                } else if eqtb.eq_level(p) != LEVEL_ONE {
                    eqtb.words[p] = saved_word;
                    eqtb.set_eq_level(p, l);
                    if tracing_restores {
                        Self::restore_trace(io, eqtb, hash, memory, p, pool_str("restoring"));
                    }
                } else if tracing_restores {
                    Self::restore_trace(io, eqtb, hash, memory, p, pool_str("retaining"));
                }
            }
        }
        eqtb.sync_io_params(io);
        Ok(after_tokens)
    }

    /// §284: `restore_trace(p, s)`.
    pub fn restore_trace(
        io: &mut IoState,
        eqtb: &Eqtb,
        hash: &HashTable,
        memory: &Memory,
        p: usize,
        s: StrNum,
    ) {
        io.begin_diagnostic();
        io.print_char(b'{');
        io.print_str(s);
        io.print_char(b' ');
        show_eqtb(io, eqtb, hash, memory, p);
        io.print_char(b'}');
        io.end_diagnostic(false);
    }

    /// §285: Search `save_stack` for equivalents that point to `p`.
    pub fn search_save_stack(&self, io: &mut IoState, p: u16) {
        for (q, word) in self.stack[..self.save_ptr].iter().enumerate() {
            if word.rh() == p {
                io.print_nl(pool_str("SAVE("));
                io.print_int(q as i32);
                io.print_char(b')');
            }
        }
    }

    /// §288: `prepare_mag`.
    pub fn prepare_mag(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hooks: &mut dyn ErrorHooks,
    ) -> Result<(), Termination> {
        let mag = eqtb.int_par(MAG_CODE);
        if self.mag_set > 0 && mag != self.mag_set {
            io.print_err(pool_str("Incompatible magnification ("));
            io.print_int(mag);
            io.print_str(pool_str(");"));
            io.print_nl(pool_str(" the previous value will be retained"));
            io.set_help(&[
                pool_str("I can handle only one magnification ratio per job. So I've"),
                pool_str("reverted to the magnification you used earlier on this run."),
            ]);
            io.int_error(self.mag_set, hooks)?;
            Self::geq_word_define(io, eqtb, eqtb.layout.int_base + MAG_CODE, self.mag_set);
        }
        let mag = eqtb.int_par(MAG_CODE);
        if mag <= 0 || mag > 32768 {
            io.print_err(pool_str("Illegal magnification has been changed to 1000"));
            io.set_help(&[pool_str(
                "The magnification ratio must be between 1 and 32768.",
            )]);
            io.int_error(mag, hooks)?;
            Self::geq_word_define(io, eqtb, eqtb.layout.int_base + MAG_CODE, 1000);
        }
        self.mag_set = eqtb.int_par(MAG_CODE);
        Ok(())
    }
}

/// §291: Append a new single-word token node after `p` (`store_new_token`).
pub fn store_new_token(
    io: &mut IoState,
    memory: &mut Memory,
    p: MemPtr,
    tok: Token,
    hooks: &mut dyn ErrorHooks,
) -> Result<MemPtr, Termination> {
    let q = memory.get_avail().map_err(|e| match e {
        MemoryError::Exhausted { capacity } => {
            io.overflow_with_hooks(StrNum::MAIN_MEMORY_SIZE, capacity as i32, hooks)
        }
        _ => io.confusion_with_hooks(pool_str("store_new_token"), hooks),
    })?;
    let _ = memory.set_head_link(p, q);
    let _ = memory.set_info(q, tok.raw());
    Ok(q)
}

/// §316: `set_trick_count` helper used by `show_token_list` (`§320`) and `show_context` (`§318`).
pub fn set_trick_count(io: &mut IoState) {
    io.first_count = io.tally;
    io.trick_count =
        io.tally + 1 + (io.config.error_line as i32) - (io.config.half_error_line as i32);
    if io.trick_count < io.config.error_line as i32 {
        io.trick_count = io.config.error_line as i32;
    }
}

/// §§292–294, 320: `show_token_list(p, q, l)`.
pub fn show_token_list(
    io: &mut IoState,
    eqtb: &Eqtb,
    hash: &HashTable,
    memory: &Memory,
    mut p: i32,
    q: i32,
    l: i32,
) {
    let mut match_chr = i32::from(b'#');
    let mut n = b'0';
    io.tally = 0;
    let stats = memory.stats();
    while p != 0 && io.tally < l {
        if p == q {
            set_trick_count(io);
        }
        if p < i32::from(stats.hi_mem_min) || p > i32::from(stats.mem_end) {
            io.print_esc(pool_str("CLOBBERED."));
            return;
        }
        let ptr = MemPtr::from_address(p as u16);
        let Ok(tok_val) = memory.info(ptr) else {
            io.print_esc(pool_str("CLOBBERED."));
            return;
        };
        if tok_val >= CS_TOKEN_FLAG {
            hash.print_cs(io, eqtb, i32::from(tok_val - CS_TOKEN_FLAG));
        } else {
            let m = (tok_val >> 8) as u8;
            let c = i32::from(tok_val & 0xff);
            match m {
                cmd::LEFT_BRACE
                | cmd::RIGHT_BRACE
                | cmd::MATH_SHIFT
                | cmd::TAB_MARK
                | cmd::SUP_MARK
                | cmd::SUB_MARK
                | cmd::SPACER
                | cmd::LETTER
                | cmd::OTHER_CHAR => {
                    io.print(c);
                }
                cmd::MAC_PARAM => {
                    io.print(c);
                    io.print(c);
                }
                cmd::OUT_PARAM => {
                    io.print(match_chr);
                    if c <= 9 {
                        io.print_char(b'0' + (c as u8));
                    } else {
                        io.print_char(b'!');
                        return;
                    }
                }
                cmd::MATCH => {
                    match_chr = c;
                    io.print(c);
                    n += 1;
                    io.print_char(n);
                    if n > b'9' {
                        return;
                    }
                }
                cmd::END_MATCH => {
                    io.print_str(pool_str("->"));
                }
                _ => {
                    io.print_esc(pool_str("BAD."));
                }
            }
        }
        let Ok(next) = memory.link(ptr) else {
            break;
        };
        p = i32::from(next.address());
    }
    if p != 0 {
        io.print_esc(pool_str("ETC."));
    }
}

/// §295: `token_show(p)`.
pub fn token_show(io: &mut IoState, eqtb: &Eqtb, hash: &HashTable, memory: &Memory, p: MemPtr) {
    if p != MemPtr::NULL {
        let Ok(first) = memory.link(p) else {
            return;
        };
        show_token_list(
            io,
            eqtb,
            hash,
            memory,
            i32::from(first.address()),
            0,
            10_000_000,
        );
    }
}

/// §699: `print_size(s)`.
pub fn print_size(io: &mut IoState, s: i32) {
    if s == i32::from(TEXT_SIZE) {
        io.print_esc(pool_str("textfont"));
    } else if s == i32::from(SCRIPT_SIZE) {
        io.print_esc(pool_str("scriptfont"));
    } else {
        io.print_esc(pool_str("scriptscriptfont"));
    }
}

/// §701: `print_style(c)`.
pub fn print_style(io: &mut IoState, c: i32) {
    match c / 2 {
        0 => io.print_esc(pool_str("displaystyle")),
        1 => io.print_esc(pool_str("textstyle")),
        2 => io.print_esc(pool_str("scriptstyle")),
        3 => io.print_esc(pool_str("scriptscriptstyle")),
        _ => io.print_str(pool_str("Unknown style!")),
    }
}

/// §298: `print_cmd_chr(cmd, chr_code)`.
pub fn print_cmd_chr(io: &mut IoState, layout: &EqtbLayout, cmd_code: u8, chr_code: i32) {
    let chr_cmd = |io: &mut IoState, label: &str| {
        io.print_str(pool_str(label));
        io.print(chr_code);
    };
    match cmd_code {
        cmd::LEFT_BRACE => chr_cmd(io, "begin-group character "),
        cmd::RIGHT_BRACE => chr_cmd(io, "end-group character "),
        cmd::MATH_SHIFT => chr_cmd(io, "math shift character "),
        cmd::MAC_PARAM => chr_cmd(io, "macro parameter character "),
        cmd::SUP_MARK => chr_cmd(io, "superscript character "),
        cmd::SUB_MARK => chr_cmd(io, "subscript character "),
        cmd::ENDV => io.print_str(pool_str("end of alignment template")),
        cmd::SPACER => chr_cmd(io, "blank space "),
        cmd::LETTER => chr_cmd(io, "the letter "),
        cmd::OTHER_CHAR => chr_cmd(io, "the character "),

        // §227: Glue parameters and registers
        cmd::ASSIGN_GLUE | cmd::ASSIGN_MU_GLUE => {
            if (chr_code as usize) < layout.skip_base {
                Eqtb::print_skip_param(io, chr_code - (layout.glue_base as i32));
            } else if (chr_code as usize) < layout.mu_skip_base {
                io.print_esc(pool_str("skip"));
                io.print_int(chr_code - (layout.skip_base as i32));
            } else {
                io.print_esc(pool_str("muskip"));
                io.print_int(chr_code - (layout.mu_skip_base as i32));
            }
        }

        // §231: Token parameters and registers
        cmd::ASSIGN_TOKS => {
            if (chr_code as usize) >= layout.toks_base {
                io.print_esc(pool_str("toks"));
                io.print_int(chr_code - (layout.toks_base as i32));
            } else {
                let c = chr_code as usize;
                let name = if c == layout.output_routine_loc {
                    "output"
                } else if c == layout.every_par_loc {
                    "everypar"
                } else if c == layout.every_math_loc {
                    "everymath"
                } else if c == layout.every_display_loc {
                    "everydisplay"
                } else if c == layout.every_hbox_loc {
                    "everyhbox"
                } else if c == layout.every_vbox_loc {
                    "everyvbox"
                } else if c == layout.every_job_loc {
                    "everyjob"
                } else if c == layout.every_cr_loc {
                    "everycr"
                } else {
                    "errhelp"
                };
                io.print_esc(pool_str(name));
            }
        }

        // §239: Integer parameters and registers
        cmd::ASSIGN_INT => {
            if (chr_code as usize) < layout.count_base {
                Eqtb::print_param(io, chr_code - (layout.int_base as i32));
            } else {
                io.print_esc(pool_str("count"));
                io.print_int(chr_code - (layout.count_base as i32));
            }
        }

        // §249: Dimension parameters and registers
        cmd::ASSIGN_DIMEN => {
            if (chr_code as usize) < layout.scaled_base {
                Eqtb::print_length_param(io, chr_code - (layout.dimen_base as i32));
            } else {
                io.print_esc(pool_str("dimen"));
                io.print_int(chr_code - (layout.scaled_base as i32));
            }
        }

        // §266: Simple primitives
        cmd::ACCENT => io.print_esc(pool_str("accent")),
        cmd::ADVANCE => io.print_esc(pool_str("advance")),
        cmd::AFTER_ASSIGNMENT => io.print_esc(pool_str("afterassignment")),
        cmd::AFTER_GROUP => io.print_esc(pool_str("aftergroup")),
        cmd::ASSIGN_FONT_DIMEN => io.print_esc(pool_str("fontdimen")),
        cmd::BEGIN_GROUP => io.print_esc(pool_str("begingroup")),
        cmd::BREAK_PENALTY => io.print_esc(pool_str("penalty")),
        cmd::CHAR_NUM => io.print_esc(pool_str("char")),
        cmd::CS_NAME => io.print_esc(pool_str("csname")),
        cmd::DEF_FONT => io.print_esc(pool_str("font")),
        cmd::DELIM_NUM => io.print_esc(pool_str("delimiter")),
        cmd::DIVIDE => io.print_esc(pool_str("divide")),
        cmd::END_CS_NAME => io.print_esc(pool_str("endcsname")),
        cmd::END_GROUP => io.print_esc(pool_str("endgroup")),
        cmd::EX_SPACE => io.print_esc(pool_str(" ")),
        cmd::EXPAND_AFTER => io.print_esc(pool_str("expandafter")),
        cmd::HALIGN => io.print_esc(pool_str("halign")),
        cmd::HRULE => io.print_esc(pool_str("hrule")),
        cmd::IGNORE_SPACES => io.print_esc(pool_str("ignorespaces")),
        cmd::INSERT => io.print_esc(pool_str("insert")),
        cmd::ITAL_CORR => io.print_esc(pool_str("/")),
        cmd::MARK => io.print_esc(pool_str("mark")),
        cmd::MATH_ACCENT => io.print_esc(pool_str("mathaccent")),
        cmd::MATH_CHAR_NUM => io.print_esc(pool_str("mathchar")),
        cmd::MATH_CHOICE => io.print_esc(pool_str("mathchoice")),
        cmd::MULTIPLY => io.print_esc(pool_str("multiply")),
        cmd::NO_ALIGN => io.print_esc(pool_str("noalign")),
        cmd::NO_BOUNDARY => io.print_esc(pool_str("noboundary")),
        cmd::NO_EXPAND => io.print_esc(pool_str("noexpand")),
        cmd::NON_SCRIPT => io.print_esc(pool_str("nonscript")),
        cmd::OMIT => io.print_esc(pool_str("omit")),
        cmd::RADICAL => io.print_esc(pool_str("radical")),
        cmd::READ_TO_CS => io.print_esc(pool_str("read")),
        cmd::RELAX => io.print_esc(pool_str("relax")),
        cmd::SET_BOX => io.print_esc(pool_str("setbox")),
        cmd::SET_PREV_GRAF => io.print_esc(pool_str("prevgraf")),
        cmd::SET_SHAPE => io.print_esc(pool_str("parshape")),
        cmd::THE => io.print_esc(pool_str("the")),
        cmd::TOKS_REGISTER => io.print_esc(pool_str("toks")),
        cmd::VADJUST => io.print_esc(pool_str("vadjust")),
        cmd::VALIGN => io.print_esc(pool_str("valign")),
        cmd::VCENTER => io.print_esc(pool_str("vcenter")),
        cmd::VRULE => io.print_esc(pool_str("vrule")),

        // §335
        cmd::PAR_END => io.print_esc(pool_str("par")),

        // §377
        cmd::INPUT => {
            if chr_code == 0 {
                io.print_esc(pool_str("input"));
            } else {
                io.print_esc(pool_str("endinput"));
            }
        }

        // §385
        cmd::TOP_BOT_MARK => match chr_code as u16 {
            FIRST_MARK_CODE => io.print_esc(pool_str("firstmark")),
            BOT_MARK_CODE => io.print_esc(pool_str("botmark")),
            SPLIT_FIRST_MARK_CODE => io.print_esc(pool_str("splitfirstmark")),
            SPLIT_BOT_MARK_CODE => io.print_esc(pool_str("splitbotmark")),
            _ => io.print_esc(pool_str("topmark")),
        },

        // §412
        cmd::REGISTER => match chr_code as u16 {
            INT_VAL => io.print_esc(pool_str("count")),
            DIMEN_VAL => io.print_esc(pool_str("dimen")),
            GLUE_VAL => io.print_esc(pool_str("skip")),
            _ => io.print_esc(pool_str("muskip")),
        },

        // §417
        cmd::SET_AUX => {
            if chr_code == VMODE {
                io.print_esc(pool_str("prevdepth"));
            } else {
                io.print_esc(pool_str("spacefactor"));
            }
        }
        cmd::SET_PAGE_INT => {
            if chr_code == 0 {
                io.print_esc(pool_str("deadcycles"));
            } else {
                io.print_esc(pool_str("insertpenalties"));
            }
        }
        cmd::SET_BOX_DIMEN => {
            if chr_code == i32::from(WIDTH_OFFSET) {
                io.print_esc(pool_str("wd"));
            } else if chr_code == i32::from(HEIGHT_OFFSET) {
                io.print_esc(pool_str("ht"));
            } else {
                io.print_esc(pool_str("dp"));
            }
        }
        cmd::LAST_ITEM => match chr_code as u16 {
            INT_VAL => io.print_esc(pool_str("lastpenalty")),
            DIMEN_VAL => io.print_esc(pool_str("lastkern")),
            GLUE_VAL => io.print_esc(pool_str("lastskip")),
            INPUT_LINE_NO_CODE => io.print_esc(pool_str("inputlineno")),
            _ => io.print_esc(pool_str("badness")),
        },

        // §469
        cmd::CONVERT => match chr_code as u16 {
            NUMBER_CODE => io.print_esc(pool_str("number")),
            ROMAN_NUMERAL_CODE => io.print_esc(pool_str("romannumeral")),
            STRING_CODE => io.print_esc(pool_str("string")),
            MEANING_CODE => io.print_esc(pool_str("meaning")),
            FONT_NAME_CODE => io.print_esc(pool_str("fontname")),
            _ => io.print_esc(pool_str("jobname")),
        },

        // §488
        cmd::IF_TEST => match chr_code as u16 {
            IF_CAT_CODE => io.print_esc(pool_str("ifcat")),
            IF_INT_CODE => io.print_esc(pool_str("ifnum")),
            IF_DIM_CODE => io.print_esc(pool_str("ifdim")),
            IF_ODD_CODE => io.print_esc(pool_str("ifodd")),
            IF_VMODE_CODE => io.print_esc(pool_str("ifvmode")),
            IF_HMODE_CODE => io.print_esc(pool_str("ifhmode")),
            IF_MMODE_CODE => io.print_esc(pool_str("ifmmode")),
            IF_INNER_CODE => io.print_esc(pool_str("ifinner")),
            IF_VOID_CODE => io.print_esc(pool_str("ifvoid")),
            IF_HBOX_CODE => io.print_esc(pool_str("ifhbox")),
            IF_VBOX_CODE => io.print_esc(pool_str("ifvbox")),
            IFX_CODE => io.print_esc(pool_str("ifx")),
            IF_EOF_CODE => io.print_esc(pool_str("ifeof")),
            IF_TRUE_CODE => io.print_esc(pool_str("iftrue")),
            IF_FALSE_CODE => io.print_esc(pool_str("iffalse")),
            IF_CASE_CODE => io.print_esc(pool_str("ifcase")),
            _ => io.print_esc(pool_str("if")),
        },

        // §492
        cmd::FI_OR_ELSE => {
            if chr_code == i32::from(FI_CODE) {
                io.print_esc(pool_str("fi"));
            } else if chr_code == i32::from(OR_CODE) {
                io.print_esc(pool_str("or"));
            } else {
                io.print_esc(pool_str("else"));
            }
        }

        // §781
        cmd::TAB_MARK => {
            if chr_code == i32::from(SPAN_CODE) {
                io.print_esc(pool_str("span"));
            } else {
                chr_cmd(io, "alignment tab character ");
            }
        }
        cmd::CAR_RET => {
            if chr_code == i32::from(CR_CODE) {
                io.print_esc(pool_str("cr"));
            } else {
                io.print_esc(pool_str("crcr"));
            }
        }

        // §984
        cmd::SET_PAGE_DIMEN => match chr_code {
            0 => io.print_esc(pool_str("pagegoal")),
            1 => io.print_esc(pool_str("pagetotal")),
            2 => io.print_esc(pool_str("pagestretch")),
            3 => io.print_esc(pool_str("pagefilstretch")),
            4 => io.print_esc(pool_str("pagefillstretch")),
            5 => io.print_esc(pool_str("pagefilllstretch")),
            6 => io.print_esc(pool_str("pageshrink")),
            _ => io.print_esc(pool_str("pagedepth")),
        },

        // §1053
        cmd::STOP => {
            if chr_code == 1 {
                io.print_esc(pool_str("dump"));
            } else {
                io.print_esc(pool_str("end"));
            }
        }

        // §1059
        cmd::HSKIP => match chr_code as u16 {
            SKIP_CODE => io.print_esc(pool_str("hskip")),
            FIL_CODE => io.print_esc(pool_str("hfil")),
            FILL_CODE => io.print_esc(pool_str("hfill")),
            SS_CODE => io.print_esc(pool_str("hss")),
            _ => io.print_esc(pool_str("hfilneg")),
        },
        cmd::VSKIP => match chr_code as u16 {
            SKIP_CODE => io.print_esc(pool_str("vskip")),
            FIL_CODE => io.print_esc(pool_str("vfil")),
            FILL_CODE => io.print_esc(pool_str("vfill")),
            SS_CODE => io.print_esc(pool_str("vss")),
            _ => io.print_esc(pool_str("vfilneg")),
        },
        cmd::MSKIP => io.print_esc(pool_str("mskip")),
        cmd::KERN => io.print_esc(pool_str("kern")),
        cmd::MKERN => io.print_esc(pool_str("mkern")),

        // §1072
        cmd::HMOVE => {
            if chr_code == 1 {
                io.print_esc(pool_str("moveleft"));
            } else {
                io.print_esc(pool_str("moveright"));
            }
        }
        cmd::VMOVE => {
            if chr_code == 1 {
                io.print_esc(pool_str("raise"));
            } else {
                io.print_esc(pool_str("lower"));
            }
        }
        cmd::MAKE_BOX => match chr_code {
            0 => io.print_esc(pool_str("box")),
            1 => io.print_esc(pool_str("copy")),
            2 => io.print_esc(pool_str("lastbox")),
            3 => io.print_esc(pool_str("vsplit")),
            4 => io.print_esc(pool_str("vtop")),
            5 => io.print_esc(pool_str("vbox")),
            _ => io.print_esc(pool_str("hbox")),
        },
        cmd::LEADER_SHIP => match chr_code as u16 {
            A_LEADERS => io.print_esc(pool_str("leaders")),
            C_LEADERS => io.print_esc(pool_str("cleaders")),
            X_LEADERS => io.print_esc(pool_str("xleaders")),
            _ => io.print_esc(pool_str("shipout")),
        },

        // §1089
        cmd::START_PAR => {
            if chr_code == 0 {
                io.print_esc(pool_str("noindent"));
            } else {
                io.print_esc(pool_str("indent"));
            }
        }

        // §1108
        cmd::REMOVE_ITEM => {
            if chr_code == i32::from(GLUE_NODE_TYPE) {
                io.print_esc(pool_str("unskip"));
            } else if chr_code == i32::from(KERN_NODE_TYPE) {
                io.print_esc(pool_str("unkern"));
            } else {
                io.print_esc(pool_str("unpenalty"));
            }
        }
        cmd::UN_HBOX => {
            if chr_code == i32::from(COPY_CODE) {
                io.print_esc(pool_str("unhcopy"));
            } else {
                io.print_esc(pool_str("unhbox"));
            }
        }
        cmd::UN_VBOX => {
            if chr_code == i32::from(COPY_CODE) {
                io.print_esc(pool_str("unvcopy"));
            } else {
                io.print_esc(pool_str("unvbox"));
            }
        }

        // §1115
        cmd::DISCRETIONARY => {
            if chr_code == 1 {
                io.print_esc(pool_str("-"));
            } else {
                io.print_esc(pool_str("discretionary"));
            }
        }

        // §1143
        cmd::EQ_NO => {
            if chr_code == 1 {
                io.print_esc(pool_str("leqno"));
            } else {
                io.print_esc(pool_str("eqno"));
            }
        }

        // §1157
        cmd::MATH_COMP => match chr_code as u16 {
            ORD_NOAD => io.print_esc(pool_str("mathord")),
            OP_NOAD => io.print_esc(pool_str("mathop")),
            BIN_NOAD => io.print_esc(pool_str("mathbin")),
            REL_NOAD => io.print_esc(pool_str("mathrel")),
            OPEN_NOAD => io.print_esc(pool_str("mathopen")),
            CLOSE_NOAD => io.print_esc(pool_str("mathclose")),
            PUNCT_NOAD => io.print_esc(pool_str("mathpunct")),
            INNER_NOAD => io.print_esc(pool_str("mathinner")),
            UNDER_NOAD => io.print_esc(pool_str("underline")),
            _ => io.print_esc(pool_str("overline")),
        },
        cmd::LIMIT_SWITCH => {
            if chr_code == i32::from(LIMITS) {
                io.print_esc(pool_str("limits"));
            } else if chr_code == i32::from(NO_LIMITS) {
                io.print_esc(pool_str("nolimits"));
            } else {
                io.print_esc(pool_str("displaylimits"));
            }
        }

        // §1170
        cmd::MATH_STYLE => print_style(io, chr_code),

        // §1179
        cmd::ABOVE => match chr_code as u16 {
            OVER_CODE => io.print_esc(pool_str("over")),
            ATOP_CODE => io.print_esc(pool_str("atop")),
            3 => io.print_esc(pool_str("abovewithdelims")),
            4 => io.print_esc(pool_str("overwithdelims")),
            5 => io.print_esc(pool_str("atopwithdelims")),
            _ => io.print_esc(pool_str("above")),
        },

        // §1189
        cmd::LEFT_RIGHT => {
            if chr_code == i32::from(LEFT_NOAD) {
                io.print_esc(pool_str("left"));
            } else {
                io.print_esc(pool_str("right"));
            }
        }

        // §1209
        cmd::PREFIX => {
            if chr_code == 1 {
                io.print_esc(pool_str("long"));
            } else if chr_code == 2 {
                io.print_esc(pool_str("outer"));
            } else {
                io.print_esc(pool_str("global"));
            }
        }
        cmd::DEF => match chr_code {
            0 => io.print_esc(pool_str("def")),
            1 => io.print_esc(pool_str("gdef")),
            2 => io.print_esc(pool_str("edef")),
            _ => io.print_esc(pool_str("xdef")),
        },

        // §1220
        cmd::LET => {
            if chr_code != 0 {
                io.print_esc(pool_str("futurelet"));
            } else {
                io.print_esc(pool_str("let"));
            }
        }

        // §1223
        cmd::SHORTHAND_DEF => match chr_code as u16 {
            CHAR_DEF_CODE => io.print_esc(pool_str("chardef")),
            MATH_CHAR_DEF_CODE => io.print_esc(pool_str("mathchardef")),
            COUNT_DEF_CODE => io.print_esc(pool_str("countdef")),
            DIMEN_DEF_CODE => io.print_esc(pool_str("dimendef")),
            SKIP_DEF_CODE => io.print_esc(pool_str("skipdef")),
            MU_SKIP_DEF_CODE => io.print_esc(pool_str("muskipdef")),
            _ => io.print_esc(pool_str("toksdef")),
        },
        cmd::CHAR_GIVEN => {
            io.print_esc(pool_str("char"));
            io.print_hex(chr_code);
        }
        cmd::MATH_GIVEN => {
            io.print_esc(pool_str("mathchar"));
            io.print_hex(chr_code);
        }

        // §1231
        cmd::DEF_CODE => {
            let c = chr_code as usize;
            if c == layout.cat_code_base {
                io.print_esc(pool_str("catcode"));
            } else if c == layout.math_code_base {
                io.print_esc(pool_str("mathcode"));
            } else if c == layout.lc_code_base {
                io.print_esc(pool_str("lccode"));
            } else if c == layout.uc_code_base {
                io.print_esc(pool_str("uccode"));
            } else if c == layout.sf_code_base {
                io.print_esc(pool_str("sfcode"));
            } else {
                io.print_esc(pool_str("delcode"));
            }
        }
        cmd::DEF_FAMILY => print_size(io, chr_code - (layout.math_font_base as i32)),

        // §1251
        cmd::HYPH_DATA => {
            if chr_code == 1 {
                io.print_esc(pool_str("patterns"));
            } else {
                io.print_esc(pool_str("hyphenation"));
            }
        }

        // §1255
        cmd::ASSIGN_FONT_INT => {
            if chr_code == 0 {
                io.print_esc(pool_str("hyphenchar"));
            } else {
                io.print_esc(pool_str("skewchar"));
            }
        }

        // §1261
        cmd::SET_FONT => {
            io.print_str(pool_str("select font "));
            if chr_code == i32::from(NULL_FONT) {
                io.slow_print(i32::from(pool_str("nullfont").0));
            }
        }

        // §1263
        cmd::SET_INTERACTION => match chr_code {
            0 => io.print_esc(pool_str("batchmode")),
            1 => io.print_esc(pool_str("nonstopmode")),
            2 => io.print_esc(pool_str("scrollmode")),
            _ => io.print_esc(pool_str("errorstopmode")),
        },

        // §1273
        cmd::IN_STREAM => {
            if chr_code == 0 {
                io.print_esc(pool_str("closein"));
            } else {
                io.print_esc(pool_str("openin"));
            }
        }

        // §1278
        cmd::MESSAGE => {
            if chr_code == 0 {
                io.print_esc(pool_str("message"));
            } else {
                io.print_esc(pool_str("errmessage"));
            }
        }

        // §1287
        cmd::CASE_SHIFT => {
            if (chr_code as usize) == layout.lc_code_base {
                io.print_esc(pool_str("lowercase"));
            } else {
                io.print_esc(pool_str("uppercase"));
            }
        }

        // §1292
        cmd::XRAY => match chr_code as u16 {
            SHOW_BOX_CODE => io.print_esc(pool_str("showbox")),
            SHOW_THE_CODE => io.print_esc(pool_str("showthe")),
            SHOW_LISTS_CODE => io.print_esc(pool_str("showlists")),
            _ => io.print_esc(pool_str("show")),
        },

        // §1295
        cmd::UNDEFINED_CS => io.print_str(pool_str("undefined")),
        cmd::CALL => io.print_str(pool_str("macro")),
        cmd::LONG_CALL => io.print_esc(pool_str("long macro")),
        cmd::OUTER_CALL => io.print_esc(pool_str("outer macro")),
        cmd::LONG_OUTER_CALL => {
            io.print_esc(pool_str("long"));
            io.print_esc(pool_str("outer macro"));
        }
        cmd::END_TEMPLATE => io.print_esc(pool_str("outer endtemplate")),

        // §1346
        cmd::EXTENSION => match chr_code as u16 {
            OPEN_NODE => io.print_esc(pool_str("openout")),
            WRITE_NODE => io.print_esc(pool_str("write")),
            CLOSE_NODE => io.print_esc(pool_str("closeout")),
            SPECIAL_NODE => io.print_esc(pool_str("special")),
            IMMEDIATE_CODE => io.print_esc(pool_str("immediate")),
            SET_LANGUAGE_CODE => io.print_esc(pool_str("setlanguage")),
            _ => io.print_str(pool_str("[unknown extension!]")),
        },

        _ => io.print_str(pool_str("[unknown command code!]")),
    }
}

/// §296: `print_meaning(cur_cmd, cur_chr, cur_mark)`.
pub fn print_meaning(
    io: &mut IoState,
    eqtb: &Eqtb,
    hash: &HashTable,
    memory: &Memory,
    cur_mark: &[MemPtr; 5],
) {
    print_cmd_chr(io, &eqtb.layout, io.cur_cmd, io.cur_chr);
    if io.cur_cmd >= cmd::CALL {
        io.print_char(b':');
        io.print_ln();
        token_show(
            io,
            eqtb,
            hash,
            memory,
            MemPtr::from_address(io.cur_chr as u16),
        );
    } else if io.cur_cmd == cmd::TOP_BOT_MARK {
        io.print_char(b':');
        io.print_ln();
        if let Some(&m) = cur_mark.get(io.cur_chr as usize) {
            token_show(io, eqtb, hash, memory, m);
        }
    }
}

/// §299: `show_cur_cmd_chr`.
pub fn show_cur_cmd_chr(io: &mut IoState, layout: &EqtbLayout, nest: &mut SemanticNest) {
    io.begin_diagnostic();
    io.print_nl(pool_str("{"));
    if nest.cur_list.mode_field != nest.shown_mode {
        SemanticNest::print_mode(io, nest.cur_list.mode_field);
        io.print_str(pool_str(": "));
        nest.shown_mode = nest.cur_list.mode_field;
    }
    print_cmd_chr(io, layout, io.cur_cmd, io.cur_chr);
    io.print_char(b'}');
    io.end_diagnostic(false);
}

/// §§223, 229, 233–235, 242, 251–252: `show_eqtb(n)`.
pub fn show_eqtb(io: &mut IoState, eqtb: &Eqtb, hash: &HashTable, memory: &Memory, n: usize) {
    let layout = &eqtb.layout;
    if n < layout.active_base {
        io.print_char(b'?');
    } else if n < layout.glue_base {
        // §223: Region 1 or 2.
        hash.sprint_cs(io, layout, n as u16);
        io.print_char(b'=');
        print_cmd_chr(io, layout, eqtb.eq_type(n), i32::from(eqtb.equiv(n)));
        if eqtb.eq_type(n) >= cmd::CALL {
            io.print_char(b':');
            let ref_ptr = MemPtr::from_address(eqtb.equiv(n));
            let first = memory.link(ref_ptr).unwrap_or(MemPtr::NULL);
            show_token_list(io, eqtb, hash, memory, i32::from(first.address()), 0, 32);
        }
    } else if n < layout.local_base {
        // §229: Region 3.
        if n < layout.skip_base {
            Eqtb::print_skip_param(io, (n - layout.glue_base) as i32);
            io.print_char(b'=');
            let unit = if n < layout.glue_base + THIN_MU_SKIP_CODE {
                pool_str("pt")
            } else {
                pool_str("mu")
            };
            io.print_spec(memory, i32::from(eqtb.equiv(n)), unit);
        } else if n < layout.mu_skip_base {
            io.print_esc(pool_str("skip"));
            io.print_int((n - layout.skip_base) as i32);
            io.print_char(b'=');
            io.print_spec(memory, i32::from(eqtb.equiv(n)), pool_str("pt"));
        } else {
            io.print_esc(pool_str("muskip"));
            io.print_int((n - layout.mu_skip_base) as i32);
            io.print_char(b'=');
            io.print_spec(memory, i32::from(eqtb.equiv(n)), pool_str("mu"));
        }
    } else if n < layout.int_base {
        // §§233–235: Region 4.
        if n == layout.par_shape_loc {
            io.print_esc(pool_str("parshape"));
            io.print_char(b'=');
            let ptr = MemPtr::from_address(eqtb.equiv(n));
            if ptr == MemPtr::NULL {
                io.print_char(b'0');
            } else {
                let count = memory.info(ptr).unwrap_or(0);
                io.print_int(i32::from(count));
            }
        } else if n < layout.toks_base {
            print_cmd_chr(io, layout, cmd::ASSIGN_TOKS, n as i32);
            io.print_char(b'=');
            let ptr = MemPtr::from_address(eqtb.equiv(n));
            if ptr != MemPtr::NULL {
                let first = memory.link(ptr).unwrap_or(MemPtr::NULL);
                show_token_list(io, eqtb, hash, memory, i32::from(first.address()), 0, 32);
            }
        } else if n < layout.box_base {
            io.print_esc(pool_str("toks"));
            io.print_int((n - layout.toks_base) as i32);
            io.print_char(b'=');
            let ptr = MemPtr::from_address(eqtb.equiv(n));
            if ptr != MemPtr::NULL {
                let first = memory.link(ptr).unwrap_or(MemPtr::NULL);
                show_token_list(io, eqtb, hash, memory, i32::from(first.address()), 0, 32);
            }
        } else if n < layout.cur_font_loc {
            io.print_esc(pool_str("box"));
            io.print_int((n - layout.box_base) as i32);
            io.print_char(b'=');
            if eqtb.equiv(n) == 0 {
                io.print_str(pool_str("void"));
            }
        } else if n < layout.cat_code_base {
            // §234: Show the font identifier in eqtb[n].
            if n == layout.cur_font_loc {
                io.print_str(pool_str("current font"));
            } else if n < layout.math_font_base + 16 {
                io.print_esc(pool_str("textfont"));
                io.print_int((n - layout.math_font_base) as i32);
            } else if n < layout.math_font_base + 32 {
                io.print_esc(pool_str("scriptfont"));
                io.print_int((n - layout.math_font_base - 16) as i32);
            } else {
                io.print_esc(pool_str("scriptscriptfont"));
                io.print_int((n - layout.math_font_base - 32) as i32);
            }
            io.print_char(b'=');
            io.print_esc(hash.font_id_text(layout, eqtb.equiv(n)));
        } else {
            // §235: Show the halfword code in eqtb[n].
            if n < layout.math_code_base {
                if n < layout.lc_code_base {
                    io.print_esc(pool_str("catcode"));
                    io.print_int((n - layout.cat_code_base) as i32);
                } else if n < layout.uc_code_base {
                    io.print_esc(pool_str("lccode"));
                    io.print_int((n - layout.lc_code_base) as i32);
                } else if n < layout.sf_code_base {
                    io.print_esc(pool_str("uccode"));
                    io.print_int((n - layout.uc_code_base) as i32);
                } else {
                    io.print_esc(pool_str("sfcode"));
                    io.print_int((n - layout.sf_code_base) as i32);
                }
                io.print_char(b'=');
                io.print_int(i32::from(eqtb.equiv(n)));
            } else {
                io.print_esc(pool_str("mathcode"));
                io.print_int((n - layout.math_code_base) as i32);
                io.print_char(b'=');
                io.print_int(i32::from(eqtb.equiv(n)));
            }
        }
    } else if n < layout.dimen_base {
        // §242: Region 5.
        if n < layout.count_base {
            Eqtb::print_param(io, (n - layout.int_base) as i32);
        } else if n < layout.del_code_base {
            io.print_esc(pool_str("count"));
            io.print_int((n - layout.count_base) as i32);
        } else {
            io.print_esc(pool_str("delcode"));
            io.print_int((n - layout.del_code_base) as i32);
        }
        io.print_char(b'=');
        io.print_int(eqtb.int(n));
    } else if n <= layout.eqtb_size {
        // §251: Region 6.
        if n < layout.scaled_base {
            Eqtb::print_length_param(io, (n - layout.dimen_base) as i32);
        } else {
            io.print_esc(pool_str("dimen"));
            io.print_int((n - layout.scaled_base) as i32);
        }
        io.print_char(b'=');
        io.print_scaled(eqtb.scaled(n));
        io.print_str(pool_str("pt"));
    } else {
        io.print_char(b'?');
    }
}
