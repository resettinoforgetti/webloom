//! Macro invocation, tex.web §§387–401. Expansion dispatch is not yet implemented.

use crate::Engine;
use crate::io::{StrNum, Termination, pool_str};
use crate::memory::MemPtr;
use crate::scanner::{MACRO, ScannerErrorHooks, ScannerStatus, TOKEN_LIST, V_TEMPLATE};
use crate::tables::{
    END_MATCH_TOKEN, LEFT_BRACE_LIMIT, MATCH_TOKEN, RIGHT_BRACE_LIMIT, SPACE_TOKEN,
    TRACING_MACROS_CODE, cmd, show_token_list, token_show,
};

fn is_match(token: u16) -> bool {
    (MATCH_TOKEN..=END_MATCH_TOKEN).contains(&token)
}

impl Engine {
    /// Invoke the compiled macro selected by `get_token` (§389).
    ///
    /// Requires `cur_cs` to name a macro and `cur_chr` to point to its valid,
    /// reference-counted definition (at most nine parameters, ending with
    /// `END_MATCH_TOKEN` before the replacement text). Arguments are read without
    /// expansion. The body is left on the input stack for subsequent token reads.
    pub fn macro_call(&mut self) -> Result<(), Termination> {
        let status = self.scanner.scanner_status;
        let warning = self.scanner.warning_index;
        self.scanner.warning_index = self.scanner.cur_cs;
        let result = self.macro_call_inner();
        self.scanner.scanner_status = status;
        self.scanner.warning_index = warning;
        result
    }

    fn macro_call_inner(&mut self) -> Result<(), Termination> {
        let reference = MemPtr::from_address(self.io.cur_chr as u16);
        let mut r = self.memory.link(reference).expect("macro reference");
        // §388: these lists stay off param_stack until all arguments are read.
        let mut parameters = Vec::with_capacity(9);
        if self.eqtb.int_par(TRACING_MACROS_CODE) > 0 {
            self.io.begin_diagnostic();
            self.io.print_ln();
            self.hash.print_cs(
                &mut self.io,
                &self.eqtb,
                i32::from(self.scanner.warning_index),
            );
            token_show(
                &mut self.io,
                &self.eqtb,
                &self.hash,
                &self.memory,
                reference,
            );
            self.io.end_diagnostic(false);
        }

        if self.memory.info(r).expect("macro parameter text") != END_MATCH_TOKEN {
            self.scanner.scanner_status = ScannerStatus::Matching;
            self.scanner.long_state = self.eqtb.eq_type(usize::from(self.scanner.cur_cs));
            if self.scanner.long_state >= cmd::OUTER_CALL {
                self.scanner.long_state -= 2;
            }
            loop {
                let head = self.memory.temp_head();
                self.memory.set_head_link(head, MemPtr::NULL).unwrap();
                let mut tail = head;
                let mut items = 0;
                let mut rbrace_ptr = MemPtr::NULL;
                let token = self.memory.info(r).unwrap();
                let mut match_chr = 0;
                let s = if (MATCH_TOKEN..END_MATCH_TOKEN).contains(&token) {
                    match_chr = (token - MATCH_TOKEN) as u8;
                    r = self.memory.link(r).unwrap();
                    r
                } else {
                    MemPtr::NULL
                };

                'scan: loop {
                    self.get_token()?;
                    if self.io.cur_tok as u16 == self.memory.info(r).unwrap() {
                        r = self.memory.link(r).unwrap();
                        if is_match(self.memory.info(r).unwrap()) {
                            // §394: a delimiter ending in #{ is scanned again in the body.
                            if self.io.cur_tok < i32::from(LEFT_BRACE_LIMIT) {
                                self.io.align_state -= 1;
                            }
                            break;
                        }
                        continue;
                    }

                    // §397: retain overlapping prefixes of a delimiter (e.g. aab
                    // while scanning aaab), contributing only the unmatched part.
                    if s != r {
                        if s == MemPtr::NULL {
                            self.io.print_err(pool_str("Use of "));
                            self.hash.sprint_cs(
                                &mut self.io,
                                &self.eqtb.layout,
                                self.scanner.warning_index,
                            );
                            self.io.print_str(pool_str(" doesn't match its definition"));
                            self.io.set_help(&[
                                pool_str(
                                    "If you say, e.g., `\\def\\a1{...}', then you must always",
                                ),
                                pool_str("put `1' after `\\a', since control sequence names are"),
                                pool_str("made up of letters only. The macro here has not been"),
                                pool_str("followed by the required stuff, so I'm ignoring it."),
                            ]);
                            return self.error();
                        }
                        let mut t = s;
                        loop {
                            self.append_argument_token(&mut tail, self.memory.info(t).unwrap())?;
                            items += 1;
                            let mut u = self.memory.link(t).unwrap();
                            let mut v = s;
                            loop {
                                if u == r {
                                    if self.io.cur_tok as u16 == self.memory.info(v).unwrap() {
                                        r = self.memory.link(v).unwrap();
                                        continue 'scan;
                                    }
                                    break;
                                }
                                if self.memory.info(u).unwrap() != self.memory.info(v).unwrap() {
                                    break;
                                }
                                u = self.memory.link(u).unwrap();
                                v = self.memory.link(v).unwrap();
                            }
                            t = self.memory.link(t).unwrap();
                            if t == r {
                                break;
                            }
                        }
                        r = s;
                    }

                    if self.io.cur_tok == i32::from(self.hash.par_token.raw())
                        && self.scanner.long_state != cmd::LONG_CALL
                    {
                        return self.abort_macro_argument(&parameters, 0);
                    }
                    if self.io.cur_tok < i32::from(RIGHT_BRACE_LIMIT) {
                        if self.io.cur_tok < i32::from(LEFT_BRACE_LIMIT) {
                            // §399: braces protect delimiter tokens inside groups.
                            let mut unbalance = 1;
                            loop {
                                self.append_argument_token(&mut tail, self.io.cur_tok as u16)?;
                                self.get_token()?;
                                if self.io.cur_tok == i32::from(self.hash.par_token.raw())
                                    && self.scanner.long_state != cmd::LONG_CALL
                                {
                                    return self.abort_macro_argument(&parameters, unbalance);
                                }
                                if self.io.cur_tok < i32::from(RIGHT_BRACE_LIMIT) {
                                    if self.io.cur_tok < i32::from(LEFT_BRACE_LIMIT) {
                                        unbalance += 1;
                                    } else {
                                        unbalance -= 1;
                                        if unbalance == 0 {
                                            break;
                                        }
                                    }
                                }
                            }
                            rbrace_ptr = tail;
                            self.append_argument_token(&mut tail, self.io.cur_tok as u16)?;
                        } else {
                            // §395: preserve the offending brace and insert \par.
                            self.scanner.back_input(
                                &mut self.io,
                                &mut self.eqtb,
                                &mut self.hash,
                                &mut self.memory,
                            )?;
                            self.io.print_err(pool_str("Argument of "));
                            self.hash.sprint_cs(
                                &mut self.io,
                                &self.eqtb.layout,
                                self.scanner.warning_index,
                            );
                            self.io.print_str(pool_str(" has an extra }"));
                            self.io.set_help(&[
                                pool_str(
                                    "I've run across a `}' that doesn't seem to match anything.",
                                ),
                                pool_str("For example, `\\def\\a#1{...}' and `\\a}' would produce"),
                                pool_str("this error. If you simply proceed now, the `\\par' that"),
                                pool_str("I've just inserted will cause me to report a runaway"),
                                pool_str("argument that might be the root of the problem. But if"),
                                pool_str(
                                    "your `}' was spurious, just type `2' and it will go away.",
                                ),
                            ]);
                            self.io.align_state += 1;
                            self.scanner.long_state = cmd::CALL;
                            self.io.cur_tok = i32::from(self.hash.par_token.raw());
                            self.scanner.ins_error(
                                &mut self.io,
                                &mut self.eqtb,
                                &mut self.hash,
                                &mut self.memory,
                            )?;
                            continue;
                        }
                    } else {
                        if self.io.cur_tok == i32::from(SPACE_TOKEN)
                            && is_match(self.memory.info(r).unwrap())
                        {
                            continue;
                        }
                        self.append_argument_token(&mut tail, self.io.cur_tok as u16)?;
                    }
                    items += 1;
                    if is_match(self.memory.info(r).unwrap()) {
                        break;
                    }
                }

                if s != MemPtr::NULL {
                    // §400: remove braces only when the entire argument is one group.
                    let first = self.memory.head_link(head).unwrap();
                    let argument =
                        if items == 1 && self.memory.info(tail).unwrap() < RIGHT_BRACE_LIMIT {
                            self.memory.set_head_link(rbrace_ptr, MemPtr::NULL).unwrap();
                            self.memory.free_avail(tail).unwrap();
                            let argument = self.memory.link(first).unwrap();
                            self.memory.free_avail(first).unwrap();
                            argument
                        } else {
                            first
                        };
                    parameters.push(argument);
                    if self.eqtb.int_par(TRACING_MACROS_CODE) > 0 {
                        self.io.begin_diagnostic();
                        self.io.print_nl(StrNum(u16::from(match_chr)));
                        self.io.print_int(parameters.len() as i32);
                        self.io.print_str(pool_str("<-"));
                        show_token_list(
                            &mut self.io,
                            &self.eqtb,
                            &self.hash,
                            &self.memory,
                            i32::from(argument.address()),
                            0,
                            1000,
                        );
                        self.io.end_diagnostic(false);
                    }
                }
                if self.memory.info(r).unwrap() == END_MATCH_TOKEN {
                    break;
                }
            }
        }

        // §390: reclaim exhausted inputs before pushing, including tail calls.
        while self.io.cur_input.state_field == TOKEN_LIST
            && self.io.cur_input.loc_field == 0
            && self.io.cur_input.index_field != V_TEMPLATE
        {
            self.scanner.end_token_list(
                &mut self.io,
                &mut self.eqtb,
                &mut self.hash,
                &mut self.memory,
            )?;
        }
        self.scanner.begin_token_list(
            &mut self.io,
            &mut self.eqtb,
            &mut self.hash,
            &mut self.memory,
            reference,
            MACRO,
        )?;
        self.io.cur_input.name_field = StrNum(self.scanner.warning_index);
        self.io.cur_input.loc_field = usize::from(self.memory.link(r).unwrap().address());
        if !parameters.is_empty() {
            let end = self.scanner.param_ptr + parameters.len();
            self.scanner.max_param_stack = self.scanner.max_param_stack.max(end);
            if self.scanner.max_param_stack > self.scanner.param_size {
                let size = self.scanner.param_size as i32;
                let mut hooks = ScannerErrorHooks {
                    scanner: &mut self.scanner,
                    eqtb: &mut self.eqtb,
                    hash: &mut self.hash,
                    memory: &mut self.memory,
                };
                return Err(self.io.overflow_with_hooks(
                    pool_str("parameter stack size"),
                    size,
                    &mut hooks,
                ));
            }
            self.scanner.param_stack[self.scanner.param_ptr..end].copy_from_slice(&parameters);
            self.scanner.param_ptr = end;
        }
        Ok(())
    }

    fn append_argument_token(&mut self, tail: &mut MemPtr, token: u16) -> Result<(), Termination> {
        let node = self.scanner.get_avail(
            &mut self.io,
            &mut self.eqtb,
            &mut self.hash,
            &mut self.memory,
        )?;
        self.memory.set_head_link(*tail, node).unwrap();
        self.memory.set_info(node, token).unwrap();
        *tail = node;
        Ok(())
    }

    /// §396: abandon collected arguments while leaving \par for the caller.
    fn abort_macro_argument(
        &mut self,
        parameters: &[MemPtr],
        unbalance: i32,
    ) -> Result<(), Termination> {
        if self.scanner.long_state == cmd::CALL {
            self.scanner
                .runaway(&mut self.io, &self.eqtb, &self.hash, &self.memory);
            self.io.print_err(pool_str("Paragraph ended before "));
            self.hash
                .sprint_cs(&mut self.io, &self.eqtb.layout, self.scanner.warning_index);
            self.io.print_str(pool_str(" was complete"));
            self.io.set_help(&[
                pool_str("I suspect you've forgotten a `}', causing me to apply this"),
                pool_str("control sequence to too much text. How can we recover?"),
                pool_str("My plan is to forget the whole thing and hope for the best."),
            ]);
            self.scanner.back_error(
                &mut self.io,
                &mut self.eqtb,
                &mut self.hash,
                &mut self.memory,
            )?;
        }
        self.io.align_state -= unbalance;
        for &argument in parameters {
            self.memory.flush_list(argument).unwrap();
        }
        let current = self.memory.head_link(self.memory.temp_head()).unwrap();
        self.memory.flush_list(current).unwrap();
        Ok(())
    }

    /// §§366–378: Expand the current expandable token (`cur_cmd > MAX_COMMAND`).
    pub fn expand(&mut self) -> Result<(), Termination> {
        let t: i32;
        let mut p: MemPtr;
        let cur_cmd = self.io.cur_cmd;
        if cur_cmd < cmd::CALL {
            match cur_cmd {
                cmd::TOP_BOT_MARK => {
                    let idx = self.io.cur_chr as usize;
                    if idx < self.scanner.cur_mark.len() {
                        let mark = self.scanner.cur_mark[idx];
                        if mark != MemPtr::NULL {
                            self.scanner.begin_token_list(
                                &mut self.io,
                                &mut self.eqtb,
                                &mut self.hash,
                                &mut self.memory,
                                mark,
                                crate::scanner::MARK_TEXT,
                            )?;
                        }
                    }
                }
                cmd::EXPAND_AFTER => {
                    // §§367–368
                    self.get_token()?;
                    t = self.io.cur_tok;
                    self.get_token()?;
                    if self.io.cur_cmd > cmd::MAX_COMMAND {
                        self.expand()?;
                    } else {
                        self.scanner.back_input(
                            &mut self.io,
                            &mut self.eqtb,
                            &mut self.hash,
                            &mut self.memory,
                        )?;
                    }
                    self.io.cur_tok = t;
                    self.scanner.back_input(
                        &mut self.io,
                        &mut self.eqtb,
                        &mut self.hash,
                        &mut self.memory,
                    )?;
                }
                cmd::NO_EXPAND => {
                    // §369
                    let save_scanner_status = self.scanner.scanner_status;
                    self.scanner.scanner_status = ScannerStatus::Normal;
                    self.get_token()?;
                    self.scanner.scanner_status = save_scanner_status;
                    t = self.io.cur_tok;
                    self.scanner.back_input(
                        &mut self.io,
                        &mut self.eqtb,
                        &mut self.hash,
                        &mut self.memory,
                    )?;
                    if t >= i32::from(crate::tables::CS_TOKEN_FLAG) {
                        p = self.memory.get_avail()?;
                        self.memory
                            .set_info(
                                p,
                                crate::tables::CS_TOKEN_FLAG
                                    + self.eqtb.layout.frozen_dont_expand as u16,
                            )
                            .unwrap();
                        self.memory.set_link(p, MemPtr::NULL).unwrap();
                        self.scanner.ins_list(
                            &mut self.io,
                            &mut self.eqtb,
                            &mut self.hash,
                            &mut self.memory,
                            p,
                        )?;
                    }
                }
                cmd::CS_NAME => {
                    // §§372–373
                    let r = self.memory.get_avail()?;
                    p = r;
                    loop {
                        self.get_x_token()?;
                        if self.scanner.cur_cs != 0 {
                            break;
                        }
                        let q = self.memory.get_avail()?;
                        self.memory.set_info(q, self.io.cur_tok as u16).unwrap();
                        self.memory.set_link(p, q).unwrap();
                        p = q;
                    }
                    if self.io.cur_cmd != cmd::END_CS_NAME {
                        self.io.print_err(pool_str("Missing \\endcsname inserted"));
                        self.scanner.back_error(
                            &mut self.io,
                            &mut self.eqtb,
                            &mut self.hash,
                            &mut self.memory,
                        )?;
                    }
                    // Build string from collected character tokens
                    let mut bytes = Vec::new();
                    let mut q = self.memory.link(r).unwrap_or(MemPtr::NULL);
                    while q != MemPtr::NULL {
                        let info = self.memory.info(q).unwrap_or(0);
                        bytes.push((info & 0xFF) as u8);
                        q = self.memory.link(q).unwrap_or(MemPtr::NULL);
                    }
                    self.memory.flush_list(r).unwrap();
                    let cs = if bytes.is_empty() {
                        self.eqtb.layout.null_cs as u16
                    } else {
                        let j = self.io.first;
                        for (idx, &b) in bytes.iter().enumerate() {
                            if j + idx < self.io.buffer.len() {
                                self.io.buffer[j + idx] = b;
                            }
                        }
                        self.hash.id_lookup(
                            &mut self.io,
                            &self.eqtb.layout,
                            j,
                            bytes.len(),
                            &mut crate::io::NoopErrorHooks,
                        )?
                    };
                    if self.eqtb.eq_type(usize::from(cs)) == cmd::UNDEFINED_CS {
                        self.save_stack.eq_define(
                            &mut self.io,
                            &mut self.eqtb,
                            &mut self.memory,
                            usize::from(cs),
                            cmd::RELAX,
                            256,
                            &mut crate::io::NoopErrorHooks,
                        )?;
                    }
                    self.io.cur_tok = i32::from(crate::tables::CS_TOKEN_FLAG) + i32::from(cs);
                    self.scanner.back_input(
                        &mut self.io,
                        &mut self.eqtb,
                        &mut self.hash,
                        &mut self.memory,
                    )?;
                }
                cmd::CONVERT => {
                    self.conv_toks()?;
                }
                cmd::THE => {
                    self.ins_the_toks()?;
                }
                cmd::IF_TEST => {
                    self.conditional()?;
                }
                cmd::FI_OR_ELSE => {
                    self.fi_or_else()?;
                }
                _ => {
                    self.io.print_err(pool_str("Undefined control sequence"));
                    self.error()?;
                }
            }
        } else if cur_cmd < cmd::END_TEMPLATE {
            self.macro_call()?;
        } else {
            // END_TEMPLATE
            self.io.cur_tok =
                i32::from(crate::tables::CS_TOKEN_FLAG) + self.eqtb.layout.frozen_endv as i32;
            self.scanner.back_input(
                &mut self.io,
                &mut self.eqtb,
                &mut self.hash,
                &mut self.memory,
            )?;
        }
        Ok(())
    }

    /// §380: Get the next non-expandable token.
    pub fn get_x_token(&mut self) -> Result<(), Termination> {
        loop {
            self.get_next()?;
            if self.io.cur_cmd <= cmd::MAX_COMMAND {
                break;
            }
            self.expand()?;
        }
        if self.scanner.cur_cs == 0 {
            self.io.cur_tok = i32::from(self.io.cur_cmd) * 256 + self.io.cur_chr;
        } else {
            self.io.cur_tok =
                i32::from(crate::tables::CS_TOKEN_FLAG) + i32::from(self.scanner.cur_cs);
        }
        Ok(())
    }

    /// §381: Like `get_x_token`, but only expands if `cur_cmd > MAX_COMMAND`.
    pub fn x_token(&mut self) -> Result<(), Termination> {
        while self.io.cur_cmd > cmd::MAX_COMMAND {
            self.expand()?;
            self.get_next()?;
        }
        if self.scanner.cur_cs == 0 {
            self.io.cur_tok = i32::from(self.io.cur_cmd) * 256 + self.io.cur_chr;
        } else {
            self.io.cur_tok =
                i32::from(crate::tables::CS_TOKEN_FLAG) + i32::from(self.scanner.cur_cs);
        }
        Ok(())
    }

    /// §§470–472: `\number`, `\romannumeral`, `\string`, `\meaning`, `\fontname`, `\jobname`.
    pub fn conv_toks(&mut self) -> Result<(), Termination> {
        let c = self.io.cur_chr as u16;
        let mut text = String::new();
        match c {
            crate::tables::NUMBER_CODE => {
                self.scan_int()?;
                text = self.scanner.cur_val.to_string();
            }
            crate::tables::ROMAN_NUMERAL_CODE => {
                self.scan_int()?;
                text = to_roman_numeral(self.scanner.cur_val);
            }
            crate::tables::STRING_CODE => {
                let save_status = self.scanner.scanner_status;
                self.scanner.scanner_status = ScannerStatus::Normal;
                self.get_token()?;
                self.scanner.scanner_status = save_status;
                if self.scanner.cur_cs == 0 {
                    text.push((self.io.cur_chr as u8) as char);
                } else {
                    let esc = self.eqtb.int_par(crate::tables::ESCAPE_CHAR_CODE);
                    if (0..=255).contains(&esc) {
                        text.push((esc as u8) as char);
                    }
                    let cs_str = self.hash.text(usize::from(self.scanner.cur_cs));
                    if cs_str != StrNum::ZERO {
                        text.push_str(&String::from_utf8_lossy(self.io.str_bytes(cs_str)));
                    }
                }
            }
            crate::tables::MEANING_CODE => {
                let save_status = self.scanner.scanner_status;
                self.scanner.scanner_status = ScannerStatus::Normal;
                self.get_token()?;
                self.scanner.scanner_status = save_status;
                if self.scanner.cur_cs == 0 {
                    text = format!("character {}", self.io.cur_chr as u8 as char);
                } else if self.io.cur_cmd >= cmd::CALL {
                    text = "macro:".to_string();
                } else {
                    text = format!("primitive({})", self.io.cur_cmd);
                }
            }
            crate::tables::FONT_NAME_CODE => {
                text = "nullfont".to_string();
            }
            crate::tables::JOB_NAME_CODE => {
                if self.io.job_name != StrNum::ZERO {
                    text =
                        String::from_utf8_lossy(self.io.str_bytes(self.io.job_name)).into_owned();
                } else {
                    text = "texput".to_string();
                }
            }
            _ => {}
        }

        let head = self.memory.temp_head();
        self.memory.set_head_link(head, MemPtr::NULL).unwrap();
        let mut tail = head;
        for &b in text.as_bytes() {
            let q = self.memory.get_avail()?;
            let tok = if b == b' ' {
                SPACE_TOKEN
            } else {
                cmd::OTHER_CHAR as u16 * 256 + u16::from(b)
            };
            self.memory.set_info(q, tok).unwrap();
            if tail == head {
                self.memory.set_head_link(head, q).unwrap();
            } else {
                self.memory.set_link(tail, q).unwrap();
            }
            tail = q;
        }
        let first = self.memory.head_link(head).unwrap_or(MemPtr::NULL);
        if first != MemPtr::NULL {
            self.scanner.ins_list(
                &mut self.io,
                &mut self.eqtb,
                &mut self.hash,
                &mut self.memory,
                first,
            )?;
        }
        Ok(())
    }

    /// §495: Push conditional state onto `cond_ptr` stack.
    fn push_cond(&mut self) -> Result<(), Termination> {
        let crate::memory::NodeAllocation::Allocated(p) = self.memory.get_node(2)? else {
            return Err(Termination::JumpOut);
        };
        self.memory
            .set_link(p, self.scanner.cond_ptr)
            .expect("cond link");
        let packed = (u16::from(self.scanner.if_limit) << 8) | u16::from(self.scanner.cur_if);
        self.memory.set_info(p, packed).expect("cond info");
        self.scanner.cond_ptr = p;
        self.scanner.cur_if = self.io.cur_chr as u8;
        self.scanner.if_limit = crate::tables::IF_CODE;
        self.scanner.if_line = self.io.line;
        Ok(())
    }

    /// §496: Pop conditional state from `cond_ptr` stack.
    fn pop_cond(&mut self) {
        let p = self.scanner.cond_ptr;
        if p == MemPtr::NULL {
            return;
        }
        let packed = self.memory.info(p).unwrap_or(0);
        self.scanner.if_limit = (packed >> 8) as u8;
        self.scanner.cur_if = (packed & 0xFF) as u8;
        self.scanner.cond_ptr = self.memory.link(p).unwrap_or(MemPtr::NULL);
        self.memory.free_node(p, 2).ok();
    }

    /// §494: Skip tokens until matching `\else`, `\or`, or `\fi` at level 0.
    pub fn pass_text(&mut self) -> Result<(), Termination> {
        let save_scanner_status = self.scanner.scanner_status;
        self.scanner.scanner_status = ScannerStatus::Skipping;
        let mut l: usize = 0;
        self.scanner.skip_line = self.io.line;
        loop {
            self.get_next()?;
            if self.io.cur_cmd == cmd::FI_OR_ELSE {
                if l == 0 {
                    break;
                }
                if self.io.cur_chr == i32::from(crate::tables::FI_CODE) {
                    l -= 1;
                }
            } else if self.io.cur_cmd == cmd::IF_TEST {
                l += 1;
            }
        }
        self.scanner.scanner_status = save_scanner_status;
        Ok(())
    }

    /// §497: Change `if_limit` for a nested or current conditional.
    pub fn change_if_limit(&mut self, l: u8, p: MemPtr) -> Result<(), Termination> {
        if p == self.scanner.cond_ptr {
            self.scanner.if_limit = l;
        } else {
            let mut q = self.scanner.cond_ptr;
            while q != MemPtr::NULL {
                let next = self.memory.link(q).unwrap_or(MemPtr::NULL);
                if next == p {
                    let cur = self.memory.info(q).unwrap_or(0) & 0xFF;
                    self.memory.set_info(q, (u16::from(l) << 8) | cur).ok();
                    return Ok(());
                }
                q = next;
            }
        }
        Ok(())
    }

    /// §§498–509: Evaluate `\if...` conditional.
    pub fn conditional(&mut self) -> Result<(), Termination> {
        self.push_cond()?;
        let save_cond_ptr = self.scanner.cond_ptr;
        let this_if = u16::from(self.scanner.cur_if);
        let b: bool = match this_if {
            crate::tables::IF_TRUE_CODE => true,
            crate::tables::IF_FALSE_CODE => false,
            crate::tables::IF_ODD_CODE => {
                self.scan_int()?;
                (self.scanner.cur_val & 1) != 0
            }
            crate::tables::IF_INT_CODE | crate::tables::IF_DIM_CODE => {
                if this_if == crate::tables::IF_INT_CODE {
                    self.scan_int()?;
                } else {
                    self.scan_dimen(false, false, false)?;
                }
                let n = self.scanner.cur_val;
                self.get_x_token_skip_spaces()?;
                let r = self.io.cur_tok;
                if this_if == crate::tables::IF_INT_CODE {
                    self.scan_int()?;
                } else {
                    self.scan_dimen(false, false, false)?;
                }
                let m = self.scanner.cur_val;
                if r == i32::from(cmd::OTHER_CHAR) * 256 + b'<' as i32 {
                    n < m
                } else if r == i32::from(cmd::OTHER_CHAR) * 256 + b'=' as i32 {
                    n == m
                } else if r == i32::from(cmd::OTHER_CHAR) * 256 + b'>' as i32 {
                    n > m
                } else {
                    self.io.print_err(pool_str("Missing = inserted for "));
                    self.error()?;
                    false
                }
            }
            crate::tables::IF_CHAR_CODE | crate::tables::IF_CAT_CODE => {
                self.get_x_token()?;
                let n = if this_if == crate::tables::IF_CHAR_CODE {
                    if self.scanner.cur_cs == 0 {
                        self.io.cur_chr
                    } else {
                        256
                    }
                } else if self.scanner.cur_cs == 0 {
                    i32::from(self.io.cur_cmd)
                } else {
                    16
                };
                self.get_x_token()?;
                let m = if this_if == crate::tables::IF_CHAR_CODE {
                    if self.scanner.cur_cs == 0 {
                        self.io.cur_chr
                    } else {
                        256
                    }
                } else if self.scanner.cur_cs == 0 {
                    i32::from(self.io.cur_cmd)
                } else {
                    16
                };
                n == m
            }
            crate::tables::IFX_CODE => {
                let save_status = self.scanner.scanner_status;
                self.scanner.scanner_status = ScannerStatus::Normal;
                self.get_next()?;
                let n = self.io.cur_cmd;
                let p = self.io.cur_chr;
                self.get_next()?;
                self.scanner.scanner_status = save_status;
                if n != self.io.cur_cmd {
                    false
                } else if n < cmd::CALL {
                    p == self.io.cur_chr
                } else if p == self.io.cur_chr {
                    true
                } else {
                    let mut p_ptr = self
                        .memory
                        .link(MemPtr::from_address(p as u16))
                        .unwrap_or(MemPtr::NULL);
                    let mut q_ptr = self
                        .memory
                        .link(MemPtr::from_address(self.io.cur_chr as u16))
                        .unwrap_or(MemPtr::NULL);
                    while p_ptr != MemPtr::NULL && q_ptr != MemPtr::NULL {
                        if self.memory.info(p_ptr) != self.memory.info(q_ptr) {
                            break;
                        }
                        p_ptr = self.memory.link(p_ptr).unwrap_or(MemPtr::NULL);
                        q_ptr = self.memory.link(q_ptr).unwrap_or(MemPtr::NULL);
                    }
                    p_ptr == MemPtr::NULL && q_ptr == MemPtr::NULL
                }
            }
            crate::tables::IF_CASE_CODE => {
                self.scan_int()?;
                let mut n = self.scanner.cur_val;
                while n != 0 {
                    self.pass_text()?;
                    if self.scanner.cond_ptr == save_cond_ptr {
                        if self.io.cur_chr == i32::from(crate::tables::OR_CODE) {
                            n -= 1;
                        } else {
                            if self.io.cur_chr == i32::from(crate::tables::FI_CODE) {
                                self.pop_cond();
                            } else {
                                self.scanner.if_limit = crate::tables::FI_CODE as u8;
                            }
                            return Ok(());
                        }
                    } else if self.io.cur_chr == i32::from(crate::tables::FI_CODE) {
                        self.pop_cond();
                    }
                }
                self.change_if_limit(crate::tables::OR_CODE as u8, save_cond_ptr)?;
                return Ok(());
            }
            _ => false,
        };

        if b {
            self.change_if_limit(crate::tables::ELSE_CODE as u8, save_cond_ptr)?;
            return Ok(());
        }

        // False branch: skip to \else or \fi
        loop {
            self.pass_text()?;
            if self.scanner.cond_ptr == save_cond_ptr {
                if self.io.cur_chr != i32::from(crate::tables::OR_CODE) {
                    break;
                }
            } else if self.io.cur_chr == i32::from(crate::tables::FI_CODE) {
                self.pop_cond();
            }
        }
        if self.io.cur_chr == i32::from(crate::tables::FI_CODE) {
            self.pop_cond();
        } else {
            self.scanner.if_limit = crate::tables::FI_CODE as u8;
        }
        Ok(())
    }

    /// §510: Handle `\else`, `\or`, `\fi` encountered during normal expansion.
    pub fn fi_or_else(&mut self) -> Result<(), Termination> {
        if self.io.cur_chr > i32::from(self.scanner.if_limit) {
            if self.scanner.if_limit == crate::tables::IF_CODE {
                self.scanner.back_input(
                    &mut self.io,
                    &mut self.eqtb,
                    &mut self.hash,
                    &mut self.memory,
                )?;
                return Ok(());
            }
            self.io.print_err(pool_str("Extra "));
            self.error()?;
            return Ok(());
        }
        while self.io.cur_chr != i32::from(crate::tables::FI_CODE) {
            self.pass_text()?;
        }
        self.pop_cond();
        Ok(())
    }
}

fn to_roman_numeral(mut n: i32) -> String {
    if n <= 0 {
        return String::new();
    }
    let table = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for &(val, sym) in &table {
        while n >= val {
            out.push_str(sym);
            n -= val;
        }
    }
    out
}
