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
}
