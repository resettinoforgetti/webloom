//! Mode-independent assignment execution and definition scanning, tex.web §§1208–1298.

use crate::Engine;
use crate::io::{Termination, pool_str};
use crate::memory::MemPtr;
use crate::tables::{DIMEN_VAL, GLUE_VAL, INT_VAL, MU_VAL, cmd};

impl Engine {
    /// §1215: Scan a control sequence or active character for definition/assignment.
    pub fn get_r_token(&mut self) -> Result<(), Termination> {
        loop {
            self.get_token()?;
            if self.io.cur_tok != i32::from(crate::tables::SPACE_TOKEN) {
                break;
            }
        }
        if self.scanner.cur_cs == 0
            || usize::from(self.scanner.cur_cs) > self.eqtb.layout.frozen_control_sequence
        {
            self.io
                .print_err(pool_str("Missing control sequence inserted"));
            self.scanner.back_error(
                &mut self.io,
                &mut self.eqtb,
                &mut self.hash,
                &mut self.memory,
            )?;
            self.scanner.cur_cs = self.eqtb.layout.frozen_protection as u16;
            self.io.cur_tok =
                i32::from(crate::tables::CS_TOKEN_FLAG) + i32::from(self.scanner.cur_cs);
        }
        Ok(())
    }

    fn define(&mut self, a: u8, p: usize, t: u8, e: i32) {
        if (a & 4) != 0 {
            crate::tables::SaveStack::geq_define(
                &mut self.io,
                &mut self.eqtb,
                &mut self.memory,
                p,
                t,
                e as u16,
            );
        } else {
            let _ = self.save_stack.eq_define(
                &mut self.io,
                &mut self.eqtb,
                &mut self.memory,
                p,
                t,
                e as u16,
                &mut crate::io::NoopErrorHooks,
            );
        }
    }

    fn word_define(&mut self, a: u8, p: usize, w: i32) {
        if (a & 4) != 0 {
            crate::tables::SaveStack::geq_word_define(&mut self.io, &mut self.eqtb, p, w);
        } else {
            let _ = self.save_stack.eq_word_define(
                &mut self.io,
                &mut self.eqtb,
                p,
                w,
                &mut crate::io::NoopErrorHooks,
            );
        }
    }

    /// §§1211–1269: Execute a prefixed command (`\global`, `\long`, `\outer`, `\def`, `\let`,
    /// `\chardef`, `\count`, `\dimen`, `\skip`, `\toks`, `\advance`, `\multiply`, `\divide`, etc.).
    pub fn prefixed_command(&mut self) -> Result<(), Termination> {
        let mut a: u8 = 0;
        while self.io.cur_cmd == cmd::PREFIX {
            let bit = self.io.cur_chr as u8;
            if (a & bit) == 0 {
                a += bit;
            }
            self.get_x_token_skip_spaces()?;
            if self.io.cur_cmd <= cmd::MAX_NON_PREFIXED_COMMAND {
                self.io.print_err(pool_str("You can't use a prefix with `"));
                self.scanner.back_error(
                    &mut self.io,
                    &mut self.eqtb,
                    &mut self.hash,
                    &mut self.memory,
                )?;
                return Ok(());
            }
        }

        match self.io.cur_cmd {
            cmd::DEF => {
                // §§1218–1220: \def, \gdef, \edef, \xdef
                let chr = self.io.cur_chr;
                if (chr & 1) != 0 {
                    // \gdef or \xdef implies \global
                    a |= 4;
                }
                let xpand = (chr & 2) != 0;
                self.get_r_token()?;
                let p = usize::from(self.scanner.cur_cs);
                let q = self.scan_toks(true, xpand)?;
                let call_cmd = cmd::CALL + (a & 3);
                self.define(a, p, call_cmd, i32::from(q.address()));
            }
            cmd::LET => {
                // §1221: \let, \futurelet
                let n = self.io.cur_chr;
                self.get_r_token()?;
                let p = usize::from(self.scanner.cur_cs);
                if n == 0 {
                    // \let
                    loop {
                        self.get_token()?;
                        if self.io.cur_cmd != cmd::SPACER {
                            break;
                        }
                    }
                    if self.io.cur_tok == i32::from(cmd::OTHER_CHAR) * 256 + b'=' as i32 {
                        self.get_token()?;
                        if self.io.cur_cmd == cmd::SPACER {
                            self.get_token()?;
                        }
                    }
                } else {
                    // \futurelet
                    self.get_token()?;
                    let q = self.io.cur_tok;
                    self.get_token()?;
                    self.io.cur_tok = q;
                    self.scanner.back_input(
                        &mut self.io,
                        &mut self.eqtb,
                        &mut self.hash,
                        &mut self.memory,
                    )?;
                }
                let t = self.io.cur_cmd;
                let e = self.io.cur_chr;
                if t >= cmd::CALL {
                    let ptr = MemPtr::from_address(e as u16);
                    if let Ok(rc) = self.memory.info(ptr) {
                        self.memory.set_info(ptr, rc.saturating_add(1)).ok();
                    }
                }
                self.define(a, p, t, e);
            }
            cmd::SHORTHAND_DEF => {
                // §1224: \chardef, \mathchardef, \countdef, \dimendef, \skipdef, \muskipdef, \toksdef
                let n = self.io.cur_chr as u16;
                self.get_r_token()?;
                let p = usize::from(self.scanner.cur_cs);
                self.define(a, p, cmd::RELAX, 256);
                self.scan_optional_equals()?;
                match n {
                    crate::tables::CHAR_DEF_CODE => {
                        self.scan_char_num()?;
                        self.define(a, p, cmd::CHAR_GIVEN, self.scanner.cur_val);
                    }
                    crate::tables::MATH_CHAR_DEF_CODE => {
                        self.scan_fifteen_bit_int()?;
                        self.define(a, p, cmd::MATH_GIVEN, self.scanner.cur_val);
                    }
                    _ => {
                        self.scan_eight_bit_int()?;
                        let idx = self.scanner.cur_val;
                        match n {
                            crate::tables::COUNT_DEF_CODE => {
                                self.define(
                                    a,
                                    p,
                                    cmd::ASSIGN_INT,
                                    (self.eqtb.layout.count_base as i32) + idx,
                                );
                            }
                            crate::tables::DIMEN_DEF_CODE => {
                                self.define(
                                    a,
                                    p,
                                    cmd::ASSIGN_DIMEN,
                                    ((self.eqtb.layout.dimen_base + 256) as i32) + idx,
                                );
                            }
                            crate::tables::SKIP_DEF_CODE => {
                                self.define(
                                    a,
                                    p,
                                    cmd::ASSIGN_GLUE,
                                    (self.eqtb.layout.skip_base as i32) + idx,
                                );
                            }
                            crate::tables::MU_SKIP_DEF_CODE => {
                                self.define(
                                    a,
                                    p,
                                    cmd::ASSIGN_MU_GLUE,
                                    ((self.eqtb.layout.skip_base + 256) as i32) + idx,
                                );
                            }
                            crate::tables::TOKS_DEF_CODE => {
                                self.define(
                                    a,
                                    p,
                                    cmd::ASSIGN_TOKS,
                                    (self.eqtb.layout.toks_base as i32) + idx,
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
            cmd::TOKS_REGISTER | cmd::ASSIGN_TOKS => {
                // §1226
                let p = if self.io.cur_cmd == cmd::TOKS_REGISTER {
                    self.scan_eight_bit_int()?;
                    self.eqtb.layout.toks_base + self.scanner.cur_val as usize
                } else {
                    self.io.cur_chr as usize
                };
                self.scan_optional_equals()?;
                self.get_x_token_skip_spaces()?;
                if self.io.cur_cmd != cmd::LEFT_BRACE
                    && (self.io.cur_cmd == cmd::TOKS_REGISTER
                        || self.io.cur_cmd == cmd::ASSIGN_TOKS)
                {
                    let q = if self.io.cur_cmd == cmd::TOKS_REGISTER {
                        self.scan_eight_bit_int()?;
                        self.eqtb.layout.toks_base + self.scanner.cur_val as usize
                    } else {
                        self.io.cur_chr as usize
                    };
                    let val = self.eqtb.equiv(q);
                    if val != 0 {
                        let ptr = MemPtr::from_address(val);
                        if let Ok(rc) = self.memory.info(ptr) {
                            self.memory.set_info(ptr, rc.saturating_add(1)).ok();
                        }
                    }
                    self.define(a, p, cmd::UNDEFINED_CS, i32::from(val));
                    return Ok(());
                }
                self.scanner.back_input(
                    &mut self.io,
                    &mut self.eqtb,
                    &mut self.hash,
                    &mut self.memory,
                )?;
                let q = self.scan_toks(false, false)?;
                if self.memory.link(q).unwrap_or(MemPtr::NULL) == MemPtr::NULL {
                    self.define(a, p, cmd::UNDEFINED_CS, 0);
                    self.memory.free_avail(q).ok();
                } else {
                    self.define(a, p, cmd::UNDEFINED_CS, i32::from(q.address()));
                }
            }
            cmd::ASSIGN_INT => {
                let p = self.io.cur_chr as usize;
                self.scan_optional_equals()?;
                self.scan_int()?;
                self.word_define(a, p, self.scanner.cur_val);
            }
            cmd::ASSIGN_DIMEN => {
                let p = self.io.cur_chr as usize;
                self.scan_optional_equals()?;
                self.scan_dimen(false, false, false)?;
                self.word_define(a, p, self.scanner.cur_val);
            }
            cmd::ASSIGN_GLUE | cmd::ASSIGN_MU_GLUE => {
                let p = self.io.cur_chr as usize;
                let level = if self.io.cur_cmd == cmd::ASSIGN_MU_GLUE {
                    MU_VAL
                } else {
                    GLUE_VAL
                };
                self.scan_optional_equals()?;
                self.scan_glue(level)?;
                self.define(a, p, cmd::GLUE_REF, self.scanner.cur_val);
            }
            cmd::REGISTER => {
                // §1234: \count, \dimen, \skip, \muskip
                let l = self.io.cur_chr as u16;
                self.scan_eight_bit_int()?;
                let idx = self.scanner.cur_val as usize;
                self.scan_optional_equals()?;
                match l {
                    INT_VAL => {
                        self.scan_int()?;
                        self.word_define(
                            a,
                            self.eqtb.layout.count_base + idx,
                            self.scanner.cur_val,
                        );
                    }
                    DIMEN_VAL => {
                        self.scan_dimen(false, false, false)?;
                        self.word_define(
                            a,
                            self.eqtb.layout.dimen_base + 256 + idx,
                            self.scanner.cur_val,
                        );
                    }
                    GLUE_VAL => {
                        self.scan_glue(GLUE_VAL)?;
                        self.define(
                            a,
                            self.eqtb.layout.skip_base + idx,
                            cmd::GLUE_REF,
                            self.scanner.cur_val,
                        );
                    }
                    MU_VAL => {
                        self.scan_glue(MU_VAL)?;
                        self.define(
                            a,
                            self.eqtb.layout.skip_base + 256 + idx,
                            cmd::GLUE_REF,
                            self.scanner.cur_val,
                        );
                    }
                    _ => {}
                }
            }
            cmd::ADVANCE => {
                // §1236
                self.do_register_arithmetic(a, ArithOp::Advance)?;
            }
            cmd::MULTIPLY => {
                // §1238
                self.do_register_arithmetic(a, ArithOp::Multiply)?;
            }
            cmd::DIVIDE => {
                // §1238
                self.do_register_arithmetic(a, ArithOp::Divide)?;
            }
            cmd::DEF_CODE => {
                // §1232: \catcode, \mathcode, \lccode, \uccode, \sfcode, \delcode
                let base = self.io.cur_chr as usize;
                self.scan_char_num()?;
                let p = base + self.scanner.cur_val as usize;
                self.scan_optional_equals()?;
                self.scan_int()?;
                self.define(a, p, cmd::DATA, self.scanner.cur_val);
            }
            cmd::SET_INTERACTION => {
                // §1264
                self.io.interaction = match self.io.cur_chr {
                    0 => crate::io::Interaction::BatchMode,
                    1 => crate::io::Interaction::NonstopMode,
                    2 => crate::io::Interaction::ScrollMode,
                    _ => crate::io::Interaction::ErrorStopMode,
                };
            }
            _ => {}
        }
        Ok(())
    }

    fn do_register_arithmetic(&mut self, a: u8, op: ArithOp) -> Result<(), Termination> {
        self.get_x_token()?;
        let (p, level) = match self.io.cur_cmd {
            cmd::ASSIGN_INT => (self.io.cur_chr as usize, INT_VAL),
            cmd::ASSIGN_DIMEN => (self.io.cur_chr as usize, DIMEN_VAL),
            cmd::REGISTER => {
                let l = self.io.cur_chr as u16;
                self.scan_eight_bit_int()?;
                let idx = self.scanner.cur_val as usize;
                let base = match l {
                    INT_VAL => self.eqtb.layout.count_base + idx,
                    DIMEN_VAL => self.eqtb.layout.dimen_base + 256 + idx,
                    GLUE_VAL => self.eqtb.layout.skip_base + idx,
                    MU_VAL => self.eqtb.layout.skip_base + 256 + idx,
                    _ => self.eqtb.layout.count_base + idx,
                };
                (base, l)
            }
            _ => {
                self.io
                    .print_err(pool_str("You can't use a prefix with this"));
                self.error()?;
                return Ok(());
            }
        };

        let _ = self.scan_keyword("by")?;

        match op {
            ArithOp::Advance => {
                if level == INT_VAL {
                    self.scan_int()?;
                    let old = self.eqtb.int(p);
                    self.word_define(a, p, old.saturating_add(self.scanner.cur_val));
                } else if level == DIMEN_VAL {
                    self.scan_dimen(false, false, false)?;
                    let old = self.eqtb.int(p);
                    self.word_define(a, p, old.saturating_add(self.scanner.cur_val));
                }
            }
            ArithOp::Multiply => {
                self.scan_int()?;
                let factor = self.scanner.cur_val;
                if level <= DIMEN_VAL {
                    let old = self.eqtb.int(p);
                    let res = self
                        .arithmetic
                        .mult_and_add(
                            factor,
                            crate::arithmetic::Scaled::from_raw(old)
                                .unwrap_or(crate::arithmetic::Scaled::ZERO),
                            crate::arithmetic::Scaled::ZERO,
                            i32::MAX,
                        )
                        .map(crate::arithmetic::Scaled::raw)
                        .unwrap_or(0);
                    self.word_define(a, p, res);
                }
            }
            ArithOp::Divide => {
                self.scan_int()?;
                let divisor = self.scanner.cur_val;
                if divisor == 0 {
                    self.io.print_err(pool_str("Arithmetic overflow"));
                    self.error()?;
                } else if level <= DIMEN_VAL {
                    let old = self.eqtb.int(p);
                    let res = self
                        .arithmetic
                        .x_over_n(
                            crate::arithmetic::Scaled::from_raw(old)
                                .unwrap_or(crate::arithmetic::Scaled::ZERO),
                            divisor,
                        )
                        .map(crate::arithmetic::Scaled::raw)
                        .unwrap_or(0);
                    self.word_define(a, p, res);
                }
            }
        }
        Ok(())
    }

    /// §1270: Execute a sequence of mode-independent assignments.
    pub fn do_assignments(&mut self) -> Result<(), Termination> {
        loop {
            self.get_x_token()?;
            if self.io.cur_cmd == cmd::SPACER || self.io.cur_cmd == cmd::RELAX {
                continue;
            }
            if self.io.cur_cmd >= cmd::MIN_INTERNAL && self.io.cur_cmd <= cmd::MAX_COMMAND {
                self.prefixed_command()?;
            } else {
                self.scanner.back_input(
                    &mut self.io,
                    &mut self.eqtb,
                    &mut self.hash,
                    &mut self.memory,
                )?;
                break;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ArithOp {
    Advance,
    Multiply,
    Divide,
}
