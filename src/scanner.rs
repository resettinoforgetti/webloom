//! Knuth, tex.web §§119–120, 300–365: Input stacks, `show_context`, `runaway`, and lexical scanner (`get_next`, `get_token`).
//! Detailed contracts: docs/semantics.md.

use crate::io::{ErrorHooks, Interaction, IoState, Selector, StrNum, Termination, pool_str};
use crate::memory::{MemPtr, Memory, MemoryError};
use crate::tables::{
    CS_TOKEN_FLAG, END_LINE_CHAR_CODE, ERROR_CONTEXT_LINES_CODE, Eqtb, HashTable, LEFT_BRACE_LIMIT,
    PAUSING_CODE, RIGHT_BRACE_LIMIT, RIGHT_BRACE_TOKEN, TRACING_MACROS_CODE, Token, cmd,
    print_cmd_chr, set_trick_count, show_token_list, token_show,
};

/// §§303, 307: Scanner `state` codes (`cur_input.state_field`).
pub const TOKEN_LIST: u8 = 0;
pub const MID_LINE: u8 = 1;
pub const SKIP_BLANKS: u8 = 2 + cmd::MAX_CHAR_CODE;
pub const NEW_LINE: u8 = 3 + cmd::MAX_CHAR_CODE + cmd::MAX_CHAR_CODE;

/// §307: Token-list input level types (`cur_input.index_field` when `state == TOKEN_LIST`).
pub const PARAMETER: u8 = 0;
pub const U_TEMPLATE: u8 = 1;
pub const V_TEMPLATE: u8 = 2;
pub const BACKED_UP: u8 = 3;
pub const INSERTED: u8 = 4;
pub const MACRO: u8 = 5;
pub const OUTPUT_TEXT: u8 = 6;
pub const EVERY_PAR_TEXT: u8 = 7;
pub const EVERY_MATH_TEXT: u8 = 8;
pub const EVERY_DISPLAY_TEXT: u8 = 9;
pub const EVERY_HBOX_TEXT: u8 = 10;
pub const EVERY_VBOX_TEXT: u8 = 11;
pub const EVERY_JOB_TEXT: u8 = 12;
pub const EVERY_CR_TEXT: u8 = 13;
pub const MARK_TEXT: u8 = 14;
pub const WRITE_TEXT: u8 = 15;

/// §358: Special `cur_chr` modifier for `\relax` produced by `\noexpand`.
pub const NO_EXPAND_FLAG: i32 = 257;

/// §305: `scanner_status` values (`normal..absorbing`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScannerStatus {
    #[default]
    Normal = 0,
    Skipping = 1,
    Defining = 2,
    Matching = 3,
    Aligning = 4,
    Absorbing = 5,
}

/// §§300–365: Lexical scanner and token-list input stack state.
#[derive(Clone, Debug)]
pub struct Scanner {
    pub scanner_status: ScannerStatus,
    pub warning_index: u16,
    pub def_ref: MemPtr,
    pub param_stack: Vec<MemPtr>,
    pub param_ptr: usize,
    pub max_param_stack: usize,
    pub param_size: usize,
    pub force_eof: bool,
    pub cur_cs: u16,
    pub long_state: u8,
    pub cur_if: u8,
    pub skip_line: i32,
    pub cur_mark: [MemPtr; 5],
    pub cond_ptr: MemPtr,
    pub if_limit: u8,
    pub if_line: i32,
    pub cur_val: i32,
    pub cur_val_level: u16,
    pub radix: u8,
    pub cur_order: u8,
}

impl Scanner {
    /// §331: Initialize scanner state (`align_state = 1000000` is initialized in `IoState`).
    pub fn new(param_size: usize, io: &mut IoState) -> Self {
        io.input_ptr = 0;
        io.max_in_stack = 0;
        io.in_open = 0;
        io.open_parens = 0;
        io.max_buf_stack = 0;
        io.first = 1;
        io.cur_input.state_field = NEW_LINE;
        io.cur_input.start_field = 1;
        io.cur_input.index_field = 0;
        io.line = 0;
        io.cur_input.name_field = StrNum::ZERO;
        io.align_state = 1_000_000;
        Self {
            scanner_status: ScannerStatus::Normal,
            warning_index: 0,
            def_ref: MemPtr::NULL,
            param_stack: vec![MemPtr::NULL; param_size + 10],
            param_ptr: 0,
            max_param_stack: 0,
            param_size,
            force_eof: false,
            cur_cs: 0,
            long_state: cmd::CALL,
            cur_if: 0,
            skip_line: 0,
            cur_mark: [MemPtr::NULL; 5],
            cond_ptr: MemPtr::NULL,
            if_limit: 0,
            if_line: 0,
            cur_val: 0,
            cur_val_level: 0,
            radix: 0,
            cur_order: 0,
        }
    }

    /// Load a single line of text into `io.buffer` at `start = 1` for scanning, appending `end_line_char`
    /// if active (`0..=255`).
    pub fn set_line_bytes(&mut self, io: &mut IoState, eqtb: &Eqtb, bytes: &[u8]) {
        eqtb.sync_io_params(io);
        let start = 1_usize;
        for (i, &b) in bytes.iter().enumerate() {
            io.buffer[start + i] = b;
        }
        io.last = start + bytes.len();
        let mut limit = io.last;
        let end_line_char = eqtb.int_par(END_LINE_CHAR_CODE);
        if !(0..=255).contains(&end_line_char) {
            limit = limit.saturating_sub(1);
        } else {
            io.buffer[limit] = end_line_char as u8;
        }
        io.first = limit + 1;
        io.cur_input.state_field = NEW_LINE;
        io.cur_input.start_field = start;
        io.cur_input.loc_field = start;
        io.cur_input.limit_field = limit;
    }

    /// §§120, 306: `runaway`. Prints runaway definition/argument/preamble/text if `scanner_status > Skipping`.
    pub fn runaway(&self, io: &mut IoState, eqtb: &Eqtb, hash: &HashTable, memory: &Memory) {
        if self.scanner_status > ScannerStatus::Skipping {
            io.print_nl(pool_str("Runaway "));
            let p = match self.scanner_status {
                ScannerStatus::Defining => {
                    io.print_str(pool_str("definition"));
                    self.def_ref
                }
                ScannerStatus::Matching => {
                    io.print_str(pool_str("argument"));
                    memory.temp_head()
                }
                ScannerStatus::Aligning => {
                    io.print_str(pool_str("preamble"));
                    memory.hold_head()
                }
                ScannerStatus::Absorbing => {
                    io.print_str(pool_str("text"));
                    self.def_ref
                }
                ScannerStatus::Normal | ScannerStatus::Skipping => unreachable!(),
            };
            io.print_char(b'?');
            io.print_ln();
            let first = memory.head_link(p).unwrap_or(MemPtr::NULL);
            show_token_list(
                io,
                eqtb,
                hash,
                memory,
                i32::from(first.address()),
                0,
                (io.config.error_line as i32) - 10,
            );
        }
    }

    /// §120: Allocate a single-word node via `memory.get_avail()`, invoking `runaway` before
    /// `overflow("main memory size", ...)` if dynamic memory is exhausted.
    pub fn get_avail(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<MemPtr, Termination> {
        match memory.get_avail() {
            Ok(p) => Ok(p),
            Err(MemoryError::Exhausted { capacity }) => {
                self.runaway(io, eqtb, hash, memory);
                let mut hooks = ScannerErrorHooks {
                    scanner: self,
                    eqtb,
                    hash,
                    memory,
                };
                Err(io.overflow_with_hooks(StrNum::MAIN_MEMORY_SIZE, capacity as i32, &mut hooks))
            }
            Err(_) => {
                let mut hooks = ScannerErrorHooks {
                    scanner: self,
                    eqtb,
                    hash,
                    memory,
                };
                Err(io.confusion_with_hooks(pool_str("get_avail"), &mut hooks))
            }
        }
    }

    /// Invoke `io.error_with_hooks` with live `ScannerErrorHooks`.
    pub fn error(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<(), Termination> {
        let mut hooks = ScannerErrorHooks {
            scanner: self,
            eqtb,
            hash,
            memory,
        };
        io.error_with_hooks(&mut hooks)
    }

    /// §§311–320: `show_context`.
    pub fn show_context(&self, io: &mut IoState, eqtb: &Eqtb, hash: &HashTable, memory: &Memory) {
        io.base_ptr = io.input_ptr;
        io.input_stack[io.base_ptr] = io.cur_input;
        let mut nn = -1_i32;
        let mut bottom_line = false;
        let error_context_lines = eqtb.int_par(ERROR_CONTEXT_LINES_CODE);
        let end_line_char = eqtb.int_par(END_LINE_CHAR_CODE);

        loop {
            io.cur_input = io.input_stack[io.base_ptr];
            if io.cur_input.state_field != TOKEN_LIST
                && (io.cur_input.name_field.0 > 17 || io.base_ptr == 0)
            {
                bottom_line = true;
            }
            if io.base_ptr == io.input_ptr || bottom_line || nn < error_context_lines {
                // §312: Display the current context.
                if io.base_ptr == io.input_ptr
                    || io.cur_input.state_field != TOKEN_LIST
                    || io.cur_input.index_field != BACKED_UP
                    || io.cur_input.loc_field != 0
                {
                    io.tally = 0;
                    let old_setting = io.selector;
                    let l: i32;
                    if io.cur_input.state_field != TOKEN_LIST {
                        // §313: Print location of current line.
                        let name = io.cur_input.name_field.0;
                        if name <= 17 {
                            if name == 0 {
                                if io.base_ptr == 0 {
                                    io.print_nl(pool_str("<*>"));
                                } else {
                                    io.print_nl(pool_str("<insert> "));
                                }
                            } else {
                                io.print_nl(pool_str("<read "));
                                if name == 17 {
                                    io.print_char(b'*');
                                } else {
                                    io.print_int(i32::from(name) - 1);
                                }
                                io.print_char(b'>');
                            }
                        } else {
                            io.print_nl(pool_str("l."));
                            let line_num = if io.base_ptr == io.input_ptr
                                || usize::from(io.cur_input.index_field) == io.in_open
                            {
                                io.line
                            } else {
                                io.line_stack
                                    .get(usize::from(io.cur_input.index_field) + 1)
                                    .copied()
                                    .unwrap_or(io.line)
                            };
                            io.print_int(line_num);
                        }
                        io.print_char(b' ');

                        // §316, §318: begin_pseudoprint and pseudoprint the line.
                        l = io.tally;
                        io.tally = 0;
                        io.selector = Selector::Pseudo;
                        io.trick_count = 1_000_000;

                        let limit = io.cur_input.limit_field;
                        let j = if i32::from(io.buffer[limit]) == end_line_char {
                            limit
                        } else {
                            limit + 1
                        };
                        if j > 0 {
                            let start = io.cur_input.start_field;
                            let loc = io.cur_input.loc_field;
                            for i in start..j {
                                if i == loc {
                                    set_trick_count(io);
                                }
                                let b = io.buffer[i];
                                io.print(i32::from(b));
                            }
                        }
                    } else {
                        // §314: Print type of token list.
                        match io.cur_input.index_field {
                            PARAMETER => io.print_nl(pool_str("<argument> ")),
                            U_TEMPLATE | V_TEMPLATE => io.print_nl(pool_str("<template> ")),
                            BACKED_UP => {
                                if io.cur_input.loc_field == 0 {
                                    io.print_nl(pool_str("<recently read> "));
                                } else {
                                    io.print_nl(pool_str("<to be read again> "));
                                }
                            }
                            INSERTED => io.print_nl(pool_str("<inserted text> ")),
                            MACRO => {
                                io.print_ln();
                                hash.print_cs(io, eqtb, i32::from(io.cur_input.name_field.0));
                            }
                            OUTPUT_TEXT => io.print_nl(pool_str("<output> ")),
                            EVERY_PAR_TEXT => io.print_nl(pool_str("<everypar> ")),
                            EVERY_MATH_TEXT => io.print_nl(pool_str("<everymath> ")),
                            EVERY_DISPLAY_TEXT => io.print_nl(pool_str("<everydisplay> ")),
                            EVERY_HBOX_TEXT => io.print_nl(pool_str("<everyhbox> ")),
                            EVERY_VBOX_TEXT => io.print_nl(pool_str("<everyvbox> ")),
                            EVERY_JOB_TEXT => io.print_nl(pool_str("<everyjob> ")),
                            EVERY_CR_TEXT => io.print_nl(pool_str("<everycr> ")),
                            MARK_TEXT => io.print_nl(pool_str("<mark> ")),
                            WRITE_TEXT => io.print_nl(pool_str("<write> ")),
                            _ => io.print_nl(pool_str("?")),
                        }

                        // §316, §319: begin_pseudoprint and pseudoprint the token list.
                        l = io.tally;
                        io.tally = 0;
                        io.selector = Selector::Pseudo;
                        io.trick_count = 1_000_000;

                        let start = io.cur_input.start_field;
                        let loc = io.cur_input.loc_field as i32;
                        let p = if io.cur_input.index_field < MACRO {
                            start as i32
                        } else {
                            let head = MemPtr::from_address(start as u16);
                            i32::from(memory.link(head).unwrap_or(MemPtr::NULL).address())
                        };
                        show_token_list(io, eqtb, hash, memory, p, loc, 100_000);
                    }

                    io.selector = old_setting;

                    // §317: Print two lines using the tricky pseudoprinted information.
                    if io.trick_count == 1_000_000 {
                        set_trick_count(io);
                    }
                    let m = if io.tally < io.trick_count {
                        io.tally - io.first_count
                    } else {
                        io.trick_count - io.first_count
                    };
                    let half_err = io.config.half_error_line as i32;
                    let err_line = io.config.error_line as i32;
                    let (p_start, n) = if l + io.first_count <= half_err {
                        (0_i32, l + io.first_count)
                    } else {
                        io.print_str(pool_str("..."));
                        (l + io.first_count - half_err + 3, half_err)
                    };
                    for q in p_start..io.first_count {
                        let ch = io.trick_buf[(q as usize) % io.config.error_line];
                        io.print_char(ch);
                    }
                    io.print_ln();
                    for _ in 1..=n {
                        io.print_char(b' ');
                    }
                    let p_end = if m + n <= err_line {
                        io.first_count + m
                    } else {
                        io.first_count + (err_line - n - 3)
                    };
                    for q in io.first_count..p_end {
                        let ch = io.trick_buf[(q as usize) % io.config.error_line];
                        io.print_char(ch);
                    }
                    if m + n > err_line {
                        io.print_str(pool_str("..."));
                    }
                    nn += 1;
                }
            } else if nn == error_context_lines {
                io.print_nl(pool_str("..."));
                nn += 1;
            }
            if bottom_line {
                break;
            }
            io.base_ptr -= 1;
        }
        io.cur_input = io.input_stack[io.input_ptr];
    }

    /// §321: `push_input`.
    pub fn push_input(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<(), Termination> {
        let mut hooks = ScannerErrorHooks {
            scanner: self,
            eqtb,
            hash,
            memory,
        };
        io.push_input(&mut hooks)
    }

    /// §322: `pop_input`.
    pub fn pop_input(io: &mut IoState) {
        io.pop_input();
    }

    /// §323: `begin_token_list(p, t)`.
    pub fn begin_token_list(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
        p: MemPtr,
        t: u8,
    ) -> Result<(), Termination> {
        self.push_input(io, eqtb, hash, memory)?;
        io.cur_input.state_field = TOKEN_LIST;
        io.cur_input.start_field = usize::from(p.address());
        io.cur_input.index_field = t;
        if t >= MACRO {
            let rc = memory.info(p).unwrap_or(0);
            let _ = memory.set_info(p, rc.saturating_add(1));
            if t == MACRO {
                io.cur_input.limit_field = self.param_ptr;
            } else {
                let next = memory.link(p).unwrap_or(MemPtr::NULL);
                io.cur_input.loc_field = usize::from(next.address());
                if eqtb.int_par(TRACING_MACROS_CODE) > 1 {
                    io.begin_diagnostic();
                    io.print_nl(StrNum::EMPTY);
                    match t {
                        MARK_TEXT => io.print_esc(pool_str("mark")),
                        WRITE_TEXT => io.print_esc(pool_str("write")),
                        _ => print_cmd_chr(
                            io,
                            &eqtb.layout,
                            cmd::ASSIGN_TOKS,
                            i32::from(t - OUTPUT_TEXT) + (eqtb.layout.output_routine_loc as i32),
                        ),
                    }
                    io.print_str(pool_str("->"));
                    token_show(io, eqtb, hash, memory, p);
                    io.end_diagnostic(false);
                }
            }
        } else {
            io.cur_input.loc_field = usize::from(p.address());
        }
        Ok(())
    }

    /// §323: `back_list(p)` macro (`begin_token_list(p, backed_up)`).
    pub fn back_list(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
        p: MemPtr,
    ) -> Result<(), Termination> {
        self.begin_token_list(io, eqtb, hash, memory, p, BACKED_UP)
    }

    /// §323: `ins_list(p)` macro (`begin_token_list(p, inserted)`).
    pub fn ins_list(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
        p: MemPtr,
    ) -> Result<(), Termination> {
        self.begin_token_list(io, eqtb, hash, memory, p, INSERTED)
    }

    /// §324: `end_token_list`.
    pub fn end_token_list(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<(), Termination> {
        let t = io.cur_input.index_field;
        let start = MemPtr::from_address(io.cur_input.start_field as u16);
        if t >= BACKED_UP {
            if t <= INSERTED {
                let _ = memory.flush_list(start);
            } else {
                let _ = memory.delete_token_ref(start);
                if t == MACRO {
                    let param_start = io.cur_input.limit_field;
                    while self.param_ptr > param_start {
                        self.param_ptr -= 1;
                        let _ = memory.flush_list(self.param_stack[self.param_ptr]);
                    }
                }
            }
        } else if t == U_TEMPLATE {
            if io.align_state > 500_000 {
                io.align_state = 0;
            } else {
                let mut hooks = ScannerErrorHooks {
                    scanner: self,
                    eqtb,
                    hash,
                    memory,
                };
                return Err(io.fatal_error_with_hooks(
                    pool_str("(interwoven alignment preambles are not allowed)"),
                    &mut hooks,
                ));
            }
        }
        io.pop_input();
        let mut hooks = ScannerErrorHooks {
            scanner: self,
            eqtb,
            hash,
            memory,
        };
        io.check_interrupt(&mut hooks)
    }

    /// §325: `back_input`.
    pub fn back_input(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<(), Termination> {
        while io.cur_input.state_field == TOKEN_LIST
            && io.cur_input.loc_field == 0
            && io.cur_input.index_field != V_TEMPLATE
        {
            self.end_token_list(io, eqtb, hash, memory)?;
        }
        let p = self.get_avail(io, eqtb, hash, memory)?;
        let tok = io.cur_tok as u16;
        let _ = memory.set_info(p, tok);
        if tok < RIGHT_BRACE_LIMIT {
            if tok < LEFT_BRACE_LIMIT {
                io.align_state -= 1;
            } else {
                io.align_state += 1;
            }
        }
        self.push_input(io, eqtb, hash, memory)?;
        io.cur_input.state_field = TOKEN_LIST;
        io.cur_input.start_field = usize::from(p.address());
        io.cur_input.index_field = BACKED_UP;
        io.cur_input.loc_field = usize::from(p.address());
        Ok(())
    }

    /// §327: `back_error`.
    pub fn back_error(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<(), Termination> {
        io.ok_to_interrupt = false;
        self.back_input(io, eqtb, hash, memory)?;
        io.ok_to_interrupt = true;
        self.error(io, eqtb, hash, memory)
    }

    /// §327: `ins_error`.
    pub fn ins_error(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<(), Termination> {
        io.ok_to_interrupt = false;
        self.back_input(io, eqtb, hash, memory)?;
        io.cur_input.index_field = INSERTED;
        io.ok_to_interrupt = true;
        self.error(io, eqtb, hash, memory)
    }

    /// §328: `begin_file_reading`.
    pub fn begin_file_reading(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<(), Termination> {
        let mut hooks = ScannerErrorHooks {
            scanner: self,
            eqtb,
            hash,
            memory,
        };
        io.begin_file_reading_with_hooks(&mut hooks)
    }

    /// §329: `end_file_reading`.
    pub fn end_file_reading(io: &mut IoState) {
        io.end_file_reading_default();
    }

    /// §330: `clear_for_error_prompt`.
    pub fn clear_for_error_prompt(io: &mut IoState) {
        io.clear_for_error_prompt_default();
    }

    /// §§336–339: `check_outer_validity`.
    pub fn check_outer_validity(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<(), Termination> {
        if self.scanner_status != ScannerStatus::Normal {
            io.deletions_allowed = false;
            // §337: Back up an outer control sequence so that it can be reread.
            if self.cur_cs != 0 {
                if io.cur_input.state_field == TOKEN_LIST
                    || io.cur_input.name_field.0 < 1
                    || io.cur_input.name_field.0 > 17
                {
                    let p = self.get_avail(io, eqtb, hash, memory)?;
                    let _ = memory.set_info(p, CS_TOKEN_FLAG + self.cur_cs);
                    self.back_list(io, eqtb, hash, memory, p)?;
                }
                io.cur_cmd = cmd::SPACER;
                io.cur_chr = i32::from(b' ');
            }
            if self.scanner_status > ScannerStatus::Skipping {
                // §§338–339: Tell the user what has run away and try to recover.
                self.runaway(io, eqtb, hash, memory);
                if self.cur_cs == 0 {
                    io.print_err(pool_str("File ended"));
                } else {
                    self.cur_cs = 0;
                    io.print_err(pool_str("Forbidden control sequence found"));
                }
                io.print_str(pool_str(" while scanning "));
                let p = self.get_avail(io, eqtb, hash, memory)?;
                match self.scanner_status {
                    ScannerStatus::Defining => {
                        io.print_str(pool_str("definition"));
                        let _ = memory.set_info(p, RIGHT_BRACE_TOKEN + u16::from(b'}'));
                    }
                    ScannerStatus::Matching => {
                        io.print_str(pool_str("use"));
                        let _ = memory.set_info(p, hash.par_token.raw());
                        self.long_state = cmd::OUTER_CALL;
                    }
                    ScannerStatus::Aligning => {
                        io.print_str(pool_str("preamble"));
                        let _ = memory.set_info(p, RIGHT_BRACE_TOKEN + u16::from(b'}'));
                        let q = p;
                        let p_cr = self.get_avail(io, eqtb, hash, memory)?;
                        let _ = memory.set_link(p_cr, q);
                        let _ =
                            memory.set_info(p_cr, CS_TOKEN_FLAG + (eqtb.layout.frozen_cr as u16));
                        io.align_state = -1_000_000;
                        self.ins_list(io, eqtb, hash, memory, p_cr)?;
                        io.print_str(pool_str(" of "));
                        hash.sprint_cs(io, &eqtb.layout, self.warning_index);
                        io.set_help(&[
                            pool_str("I suspect you have forgotten a `}', causing me"),
                            pool_str("to read past where you wanted me to stop."),
                            pool_str("I'll try to recover; but if the error is serious,"),
                            pool_str("you'd better type `E' or `X' now and fix your file."),
                        ]);
                        self.error(io, eqtb, hash, memory)?;
                        io.deletions_allowed = true;
                        return Ok(());
                    }
                    ScannerStatus::Absorbing => {
                        io.print_str(pool_str("text"));
                        let _ = memory.set_info(p, RIGHT_BRACE_TOKEN + u16::from(b'}'));
                    }
                    ScannerStatus::Normal | ScannerStatus::Skipping => unreachable!(),
                }
                self.ins_list(io, eqtb, hash, memory, p)?;
                io.print_str(pool_str(" of "));
                hash.sprint_cs(io, &eqtb.layout, self.warning_index);
                io.set_help(&[
                    pool_str("I suspect you have forgotten a `}', causing me"),
                    pool_str("to read past where you wanted me to stop."),
                    pool_str("I'll try to recover; but if the error is serious,"),
                    pool_str("you'd better type `E' or `X' now and fix your file."),
                ]);
                self.error(io, eqtb, hash, memory)?;
            } else {
                // §336: Incomplete \if...
                io.print_err(pool_str("Incomplete "));
                print_cmd_chr(io, &eqtb.layout, cmd::IF_TEST, i32::from(self.cur_if));
                io.print_str(pool_str("; all text was ignored after line "));
                io.print_int(self.skip_line);
                io.set_help(&[
                    pool_str("A forbidden control sequence occurred in skipped text."),
                    pool_str("This kind of error happens when you say `\\if...' and forget"),
                    pool_str("the matching `\\fi'. I've inserted a `\\fi'; this might work."),
                ]);
                if self.cur_cs != 0 {
                    self.cur_cs = 0;
                } else {
                    io.help_line[2] =
                        pool_str("The file ended while I was skipping conditional text.");
                }
                io.cur_tok = i32::from(CS_TOKEN_FLAG) + (eqtb.layout.frozen_fi as i32);
                self.ins_error(io, eqtb, hash, memory)?;
            }
            io.deletions_allowed = true;
        }
        Ok(())
    }

    /// §§341–362: `get_next`. Sets `io.cur_cmd`, `io.cur_chr`, and `self.cur_cs`.
    pub fn get_next(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<(), Termination> {
        eqtb.sync_io_params(io);
        'restart: loop {
            self.cur_cs = 0;
            if io.cur_input.state_field != TOKEN_LIST {
                // §§343–356, 360–362: Input from external file.
                'switch: loop {
                    if io.cur_input.loc_field <= io.cur_input.limit_field {
                        io.cur_chr = i32::from(io.buffer[io.cur_input.loc_field]);
                        io.cur_input.loc_field += 1;
                        'reswitch: loop {
                            io.cur_cmd = eqtb.cat_code(io.cur_chr as u8);
                            let state = io.cur_input.state_field;
                            match io.cur_cmd {
                                cmd::IGNORE => continue 'switch,
                                cmd::SPACER if state == SKIP_BLANKS || state == NEW_LINE => {
                                    continue 'switch;
                                }
                                cmd::ESCAPE => {
                                    // §§354–356: Scan a control sequence.
                                    if io.cur_input.loc_field > io.cur_input.limit_field {
                                        self.cur_cs = eqtb.layout.null_cs as u16;
                                    } else {
                                        'start_cs: loop {
                                            let mut k = io.cur_input.loc_field;
                                            io.cur_chr = i32::from(io.buffer[k]);
                                            let mut cat = eqtb.cat_code(io.cur_chr as u8);
                                            k += 1;
                                            if cat == cmd::LETTER || cat == cmd::SPACER {
                                                io.cur_input.state_field = SKIP_BLANKS;
                                            } else {
                                                io.cur_input.state_field = MID_LINE;
                                            }
                                            if cat == cmd::LETTER && k <= io.cur_input.limit_field {
                                                // §356: Scan ahead in the buffer until finding a nonletter.
                                                loop {
                                                    io.cur_chr = i32::from(io.buffer[k]);
                                                    cat = eqtb.cat_code(io.cur_chr as u8);
                                                    k += 1;
                                                    if cat != cmd::LETTER
                                                        || k > io.cur_input.limit_field
                                                    {
                                                        break;
                                                    }
                                                }
                                                // §355: If an expanded code is present, reduce it and goto start_cs.
                                                if reduce_expanded_code_in_cs(
                                                    io,
                                                    k,
                                                    io.cur_chr as u8,
                                                    cat,
                                                ) {
                                                    continue 'start_cs;
                                                }
                                                if cat != cmd::LETTER {
                                                    k -= 1;
                                                }
                                                if k > io.cur_input.loc_field + 1 {
                                                    let loc = io.cur_input.loc_field;
                                                    let len = k - loc;
                                                    let cs = match hash.try_id_lookup(
                                                        io,
                                                        &eqtb.layout,
                                                        loc,
                                                        len,
                                                    ) {
                                                        Ok(cs) => cs,
                                                        Err((s, n)) => {
                                                            let mut hooks = ScannerErrorHooks {
                                                                scanner: self,
                                                                eqtb,
                                                                hash,
                                                                memory,
                                                            };
                                                            return Err(io.overflow_with_hooks(
                                                                s, n, &mut hooks,
                                                            ));
                                                        }
                                                    };
                                                    self.cur_cs = cs;
                                                    io.cur_input.loc_field = k;
                                                    break 'start_cs;
                                                }
                                            } else if reduce_expanded_code_in_cs(
                                                io,
                                                k,
                                                io.cur_chr as u8,
                                                cat,
                                            ) {
                                                continue 'start_cs;
                                            }
                                            self.cur_cs = (eqtb.layout.single_base as u16)
                                                + u16::from(io.buffer[io.cur_input.loc_field]);
                                            io.cur_input.loc_field += 1;
                                            break 'start_cs;
                                        }
                                    }
                                    io.cur_cmd = eqtb.eq_type(usize::from(self.cur_cs));
                                    io.cur_chr = i32::from(eqtb.equiv(usize::from(self.cur_cs)));
                                    if io.cur_cmd >= cmd::OUTER_CALL {
                                        self.check_outer_validity(io, eqtb, hash, memory)?;
                                    }
                                    break 'switch;
                                }
                                cmd::ACTIVE_CHAR => {
                                    // §353: Process an active-character control sequence.
                                    self.cur_cs =
                                        (io.cur_chr as u16) + (eqtb.layout.active_base as u16);
                                    io.cur_cmd = eqtb.eq_type(usize::from(self.cur_cs));
                                    io.cur_chr = i32::from(eqtb.equiv(usize::from(self.cur_cs)));
                                    io.cur_input.state_field = MID_LINE;
                                    if io.cur_cmd >= cmd::OUTER_CALL {
                                        self.check_outer_validity(io, eqtb, hash, memory)?;
                                    }
                                    break 'switch;
                                }
                                cmd::SUP_MARK => {
                                    // §352: If this sup_mark starts an expanded character, goto reswitch.
                                    let loc = io.cur_input.loc_field;
                                    let limit = io.cur_input.limit_field;
                                    if loc < limit && io.cur_chr == i32::from(io.buffer[loc]) {
                                        let c = io.buffer[loc + 1];
                                        if c < 128 {
                                            io.cur_input.loc_field = loc + 2;
                                            if is_hex(c) && io.cur_input.loc_field <= limit {
                                                let cc = io.buffer[io.cur_input.loc_field];
                                                if is_hex(cc) {
                                                    io.cur_input.loc_field += 1;
                                                    io.cur_chr = i32::from(hex_pair_to_byte(c, cc));
                                                    continue 'reswitch;
                                                }
                                            }
                                            if c < 64 {
                                                io.cur_chr = i32::from(c + 64);
                                            } else {
                                                io.cur_chr = i32::from(c - 64);
                                            }
                                            continue 'reswitch;
                                        }
                                    }
                                    io.cur_input.state_field = MID_LINE;
                                    break 'switch;
                                }
                                cmd::INVALID_CHAR => {
                                    // §346: Decry the invalid character and goto restart.
                                    io.print_err(pool_str(
                                        "Text line contains an invalid character",
                                    ));
                                    io.set_help(&[
                                        pool_str(
                                            "A funny symbol that I can't read has just been input.",
                                        ),
                                        pool_str(
                                            "Continue, and I'll forget that it ever happened.",
                                        ),
                                    ]);
                                    io.deletions_allowed = false;
                                    self.error(io, eqtb, hash, memory)?;
                                    io.deletions_allowed = true;
                                    continue 'restart;
                                }
                                cmd::SPACER => {
                                    // §349: mid_line + spacer -> Enter skip_blanks state, emit a space.
                                    io.cur_input.state_field = SKIP_BLANKS;
                                    io.cur_chr = i32::from(b' ');
                                    break 'switch;
                                }
                                cmd::CAR_RET if state == MID_LINE => {
                                    // §348: Finish line, emit a space.
                                    io.cur_input.loc_field = io.cur_input.limit_field + 1;
                                    io.cur_cmd = cmd::SPACER;
                                    io.cur_chr = i32::from(b' ');
                                    break 'switch;
                                }
                                cmd::CAR_RET if state == SKIP_BLANKS => {
                                    // §350: Finish line, goto switch.
                                    io.cur_input.loc_field = io.cur_input.limit_field + 1;
                                    continue 'switch;
                                }
                                cmd::COMMENT => {
                                    // §350: Finish line, goto switch.
                                    io.cur_input.loc_field = io.cur_input.limit_field + 1;
                                    continue 'switch;
                                }
                                cmd::CAR_RET if state == NEW_LINE => {
                                    // §351: Finish line, emit a \par.
                                    io.cur_input.loc_field = io.cur_input.limit_field + 1;
                                    self.cur_cs = hash.par_loc;
                                    io.cur_cmd = eqtb.eq_type(usize::from(self.cur_cs));
                                    io.cur_chr = i32::from(eqtb.equiv(usize::from(self.cur_cs)));
                                    if io.cur_cmd >= cmd::OUTER_CALL {
                                        self.check_outer_validity(io, eqtb, hash, memory)?;
                                    }
                                    break 'switch;
                                }
                                cmd::LEFT_BRACE => {
                                    if state != MID_LINE {
                                        io.cur_input.state_field = MID_LINE;
                                    }
                                    io.align_state += 1;
                                    break 'switch;
                                }
                                cmd::RIGHT_BRACE => {
                                    if state != MID_LINE {
                                        io.cur_input.state_field = MID_LINE;
                                    }
                                    io.align_state -= 1;
                                    break 'switch;
                                }
                                cmd::MATH_SHIFT
                                | cmd::TAB_MARK
                                | cmd::MAC_PARAM
                                | cmd::SUB_MARK
                                | cmd::LETTER
                                | cmd::OTHER_CHAR => {
                                    if state != MID_LINE {
                                        io.cur_input.state_field = MID_LINE;
                                    }
                                    break 'switch;
                                }
                                _ => break 'switch,
                            }
                        }
                    } else {
                        // §§360–362: Move to next line of file, or goto restart, or return if \read line finished.
                        io.cur_input.state_field = NEW_LINE;
                        let name = io.cur_input.name_field.0;
                        let end_line_char = eqtb.int_par(END_LINE_CHAR_CODE);
                        let end_line_char_inactive = !(0..=255).contains(&end_line_char);
                        if name > 17 {
                            // §362: Read next line of file into buffer, or goto restart if file ended.
                            io.line += 1;
                            io.first = io.cur_input.start_field;
                            if !self.force_eof {
                                let idx = usize::from(io.cur_input.index_field);
                                let mut file = std::mem::take(&mut io.input_file[idx]);
                                let mut hooks = ScannerErrorHooks {
                                    scanner: self,
                                    eqtb,
                                    hash,
                                    memory,
                                };
                                let read_res = io.input_ln_with_hooks(&mut file, true, &mut hooks);
                                io.input_file[idx] = file;
                                if read_res? {
                                    io.pausing = eqtb.int_par(PAUSING_CODE);
                                    let mut hooks = ScannerErrorHooks {
                                        scanner: self,
                                        eqtb,
                                        hash,
                                        memory,
                                    };
                                    io.firm_up_the_line(&mut hooks)?;
                                } else {
                                    self.force_eof = true;
                                }
                            }
                            if self.force_eof {
                                io.print_char(b')');
                                io.open_parens = io.open_parens.saturating_sub(1);
                                io.update_terminal();
                                self.force_eof = false;
                                Self::end_file_reading(io);
                                self.check_outer_validity(io, eqtb, hash, memory)?;
                                continue 'restart;
                            }
                            if end_line_char_inactive {
                                io.cur_input.limit_field =
                                    io.cur_input.limit_field.saturating_sub(1);
                            } else {
                                io.buffer[io.cur_input.limit_field] = end_line_char as u8;
                            }
                            io.first = io.cur_input.limit_field + 1;
                            io.cur_input.loc_field = io.cur_input.start_field;
                        } else {
                            if name != 0 {
                                // §360: \read line has ended.
                                io.cur_cmd = 0;
                                io.cur_chr = 0;
                                return Ok(());
                            }
                            if io.input_ptr > 0 {
                                Self::end_file_reading(io);
                                continue 'restart;
                            }
                            if io.selector.code() < Selector::LogOnly.code() {
                                let mut hooks = ScannerErrorHooks {
                                    scanner: self,
                                    eqtb,
                                    hash,
                                    memory,
                                };
                                io.open_log_file_with_hooks(&mut hooks)?;
                            }
                            if io.interaction > Interaction::NonstopMode {
                                if end_line_char_inactive {
                                    io.cur_input.limit_field += 1;
                                }
                                if io.cur_input.limit_field == io.cur_input.start_field {
                                    io.print_nl(pool_str("(Please type a command or say `\\end')"));
                                }
                                io.print_ln();
                                io.first = io.cur_input.start_field;
                                {
                                    let mut hooks = ScannerErrorHooks {
                                        scanner: self,
                                        eqtb,
                                        hash,
                                        memory,
                                    };
                                    io.prompt_input(pool_str("*"), &mut hooks)?;
                                }
                                io.cur_input.limit_field = io.last;
                                if end_line_char_inactive {
                                    io.cur_input.limit_field =
                                        io.cur_input.limit_field.saturating_sub(1);
                                } else {
                                    io.buffer[io.cur_input.limit_field] = end_line_char as u8;
                                }
                                io.first = io.cur_input.limit_field + 1;
                                io.cur_input.loc_field = io.cur_input.start_field;
                            } else {
                                let mut hooks = ScannerErrorHooks {
                                    scanner: self,
                                    eqtb,
                                    hash,
                                    memory,
                                };
                                return Err(io.fatal_error_with_hooks(
                                    pool_str("*** (job aborted, no legal \\end found)"),
                                    &mut hooks,
                                ));
                            }
                        }
                        {
                            let mut hooks = ScannerErrorHooks {
                                scanner: self,
                                eqtb,
                                hash,
                                memory,
                            };
                            io.check_interrupt(&mut hooks)?;
                        }
                        continue 'switch;
                    }
                }
            } else {
                // §§357–359: Input from token list.
                if io.cur_input.loc_field != 0 {
                    let loc_ptr = MemPtr::from_address(io.cur_input.loc_field as u16);
                    let t = memory.info(loc_ptr).unwrap_or(0);
                    let next = memory.link(loc_ptr).unwrap_or(MemPtr::NULL);
                    io.cur_input.loc_field = usize::from(next.address());
                    if t >= CS_TOKEN_FLAG {
                        self.cur_cs = t - CS_TOKEN_FLAG;
                        io.cur_cmd = eqtb.eq_type(usize::from(self.cur_cs));
                        io.cur_chr = i32::from(eqtb.equiv(usize::from(self.cur_cs)));
                        if io.cur_cmd >= cmd::OUTER_CALL {
                            if io.cur_cmd == cmd::DONT_EXPAND {
                                // §358: Get the next token, suppressing expansion.
                                let next_loc = MemPtr::from_address(io.cur_input.loc_field as u16);
                                let next_tok = memory.info(next_loc).unwrap_or(CS_TOKEN_FLAG);
                                self.cur_cs = next_tok - CS_TOKEN_FLAG;
                                io.cur_input.loc_field = 0;
                                io.cur_cmd = eqtb.eq_type(usize::from(self.cur_cs));
                                io.cur_chr = i32::from(eqtb.equiv(usize::from(self.cur_cs)));
                                if io.cur_cmd > cmd::MAX_COMMAND {
                                    io.cur_cmd = cmd::RELAX;
                                    io.cur_chr = NO_EXPAND_FLAG;
                                }
                            } else {
                                self.check_outer_validity(io, eqtb, hash, memory)?;
                            }
                        }
                    } else {
                        io.cur_cmd = (t >> 8) as u8;
                        io.cur_chr = i32::from(t & 0xff);
                        match io.cur_cmd {
                            cmd::LEFT_BRACE => io.align_state += 1,
                            cmd::RIGHT_BRACE => io.align_state -= 1,
                            cmd::OUT_PARAM => {
                                // §359: Insert macro parameter and goto restart.
                                let param_idx = io.cur_input.limit_field + (io.cur_chr as usize);
                                let param_list = self.param_stack[param_idx];
                                self.begin_token_list(
                                    io, eqtb, hash, memory, param_list, PARAMETER,
                                )?;
                                continue 'restart;
                            }
                            _ => {}
                        }
                    }
                } else {
                    self.end_token_list(io, eqtb, hash, memory)?;
                    continue 'restart;
                }
            }

            // §342: If an alignment entry has just ended, take appropriate action.
            if io.cur_cmd <= cmd::CAR_RET && io.cur_cmd >= cmd::TAB_MARK && io.align_state == 0 {
                let mut hooks = ScannerErrorHooks {
                    scanner: self,
                    eqtb,
                    hash,
                    memory,
                };
                return Err(io.fatal_error_with_hooks(
                    pool_str("(interwoven alignment preambles are not allowed)"),
                    &mut hooks,
                ));
            }
            return Ok(());
        }
    }

    /// §365: `get_token`. Sets `io.cur_cmd`, `io.cur_chr`, `self.cur_cs`, and `io.cur_tok`.
    pub fn get_token(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<(), Termination> {
        hash.no_new_control_sequence = false;
        let res = self.get_next(io, eqtb, hash, memory);
        hash.no_new_control_sequence = true;
        res?;
        if self.cur_cs == 0 {
            io.cur_tok = i32::from(io.cur_cmd) * 256 + io.cur_chr;
        } else {
            io.cur_tok = i32::from(CS_TOKEN_FLAG + self.cur_cs);
        }
        Ok(())
    }

    ///Convenience wrapper returning `Token(io.cur_tok as u16)`.
    pub fn next_token(
        &mut self,
        io: &mut IoState,
        eqtb: &mut Eqtb,
        hash: &mut HashTable,
        memory: &mut Memory,
    ) -> Result<Token, Termination> {
        self.get_token(io, eqtb, hash, memory)?;
        Ok(Token(io.cur_tok as u16))
    }
}

/// §352: `is_hex(c)` (`'0'..='9'` or `'a'..='f'`).
pub const fn is_hex(c: u8) -> bool {
    (c >= b'0' && c <= b'9') || (c >= b'a' && c <= b'f')
}

/// §352: `hex_to_cur_chr` for two lowercase hex digits `c`, `cc`.
pub const fn hex_pair_to_byte(c: u8, cc: u8) -> u8 {
    let hi = if c <= b'9' { c - b'0' } else { c - b'a' + 10 };
    let lo = if cc <= b'9' {
        cc - b'0'
    } else {
        cc - b'a' + 10
    };
    16 * hi + lo
}

/// §355: If an expanded code `^^c` or `^^cc` is present at `buffer[k - 1 ..]`, reduce it in place
/// and shift `buffer` left by `d` (`2` or `3`), returning `true` to jump to `start_cs`.
fn reduce_expanded_code_in_cs(io: &mut IoState, mut k: usize, cur_chr: u8, cat: u8) -> bool {
    let limit = io.cur_input.limit_field;
    if io.buffer[k] == cur_chr && cat == cmd::SUP_MARK && k < limit {
        let c = io.buffer[k + 1];
        if c < 128 {
            let mut d = 2_usize;
            if is_hex(c) && k + 2 <= limit {
                let cc = io.buffer[k + 2];
                if is_hex(cc) {
                    d += 1;
                }
            }
            if d > 2 {
                let cc = io.buffer[k + 2];
                io.buffer[k - 1] = hex_pair_to_byte(c, cc);
            } else if c < 64 {
                io.buffer[k - 1] = c + 64;
            } else {
                io.buffer[k - 1] = c - 64;
            }
            io.cur_input.limit_field -= d;
            io.first -= d;
            while k <= io.cur_input.limit_field {
                io.buffer[k] = io.buffer[k + d];
                k += 1;
            }
            return true;
        }
    }
    false
}

/// §78, §§311–365, §1284: Live `ErrorHooks` adapter connecting `IoState::error_with_hooks` to
/// `Scanner`, `Eqtb`, `HashTable`, and `Memory`.
pub struct ScannerErrorHooks<'a> {
    pub scanner: &'a mut Scanner,
    pub eqtb: &'a mut Eqtb,
    pub hash: &'a mut HashTable,
    pub memory: &'a mut Memory,
}

impl ErrorHooks for ScannerErrorHooks<'_> {
    fn show_context(&mut self, io: &mut IoState) {
        self.scanner
            .show_context(io, self.eqtb, self.hash, self.memory);
    }

    fn clear_for_error_prompt(&mut self, io: &mut IoState) {
        Scanner::clear_for_error_prompt(io);
    }

    fn get_token(&mut self, io: &mut IoState) -> Result<(), Termination> {
        self.scanner
            .get_token(io, self.eqtb, self.hash, self.memory)
    }

    fn begin_file_reading(&mut self, io: &mut IoState) -> Result<(), Termination> {
        self.scanner
            .begin_file_reading(io, self.eqtb, self.hash, self.memory)
    }

    fn end_file_reading(&mut self, io: &mut IoState) {
        Scanner::end_file_reading(io);
    }

    fn give_err_help(&mut self, io: &mut IoState) {
        // §1284: give_err_help
        let err_help_ptr = MemPtr::from_address(self.eqtb.equiv(self.eqtb.layout.err_help_loc));
        token_show(io, self.eqtb, self.hash, self.memory, err_help_ptr);
    }
}
