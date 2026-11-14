//! Basic value scanners and token-list builders, tex.web §§402–486, §526.

use crate::Engine;
use crate::arithmetic::{Scaled, round_decimals};
use crate::io::{Termination, pool_str};
use crate::memory::{GLUE_SPEC_SIZE, MemPtr};
use crate::scanner::ScannerStatus;
use crate::tables::{
    CS_TOKEN_FLAG, END_MATCH_TOKEN, INT_VAL, LEFT_BRACE_LIMIT, MATCH_TOKEN, MU_VAL,
    OUT_PARAM_TOKEN, RIGHT_BRACE_LIMIT, TOK_VAL, cmd,
};

const INFINITY: i32 = 0x7FFF_FFFF;
const MAX_DIMEN: i32 = 0x3FFF_FFFF;
const ZERO_TOKEN: u16 = (cmd::OTHER_CHAR as u16) * 256 + (b'0' as u16);
const NORMAL: u8 = 0;
const FIL: u8 = 1;
const FILLL: u8 = 3;

const RADIX_OCTAL: u8 = 8;
const RADIX_DECIMAL: u8 = 10;
const RADIX_HEX: u8 = 16;

impl Engine {
    /// §403: Scan a left brace, skipping spaces and `\relax`.
    pub fn scan_left_brace(&mut self) -> Result<(), Termination> {
        loop {
            self.get_x_token()?;
            if self.io.cur_cmd != cmd::SPACER && self.io.cur_cmd != cmd::RELAX {
                break;
            }
        }
        if self.io.cur_cmd != cmd::LEFT_BRACE {
            self.io.print_err(pool_str("Missing { inserted"));
            self.scanner.back_error(
                &mut self.io,
                &mut self.eqtb,
                &mut self.hash,
                &mut self.memory,
            )?;
            self.io.cur_tok = i32::from(LEFT_BRACE_LIMIT) - 1;
            self.io.cur_cmd = cmd::LEFT_BRACE;
            self.io.cur_chr = b'{' as i32;
            self.io.align_state += 1;
        }
        Ok(())
    }

    /// §405: Scan an optional `=` sign, skipping leading spaces.
    pub fn scan_optional_equals(&mut self) -> Result<(), Termination> {
        loop {
            self.get_x_token()?;
            if self.io.cur_cmd != cmd::SPACER {
                break;
            }
        }
        if self.io.cur_tok != i32::from(cmd::OTHER_CHAR) * 256 + b'=' as i32 {
            self.scanner.back_input(
                &mut self.io,
                &mut self.eqtb,
                &mut self.hash,
                &mut self.memory,
            )?;
        }
        Ok(())
    }

    /// §407: Scan a keyword (case-insensitive ASCII), returning `true` if matched.
    pub fn scan_keyword(&mut self, s: &str) -> Result<bool, Termination> {
        let head = self.memory.temp_head();
        self.memory.set_head_link(head, MemPtr::NULL).unwrap();
        let mut p = head;
        let bytes = s.as_bytes();
        let mut k = 0usize;
        while k < bytes.len() {
            self.get_x_token()?;
            let c = bytes[k];
            let alt = if c.is_ascii_lowercase() {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            };
            if self.scanner.cur_cs == 0
                && (self.io.cur_chr == i32::from(c) || self.io.cur_chr == i32::from(alt))
            {
                let q = self.memory.get_avail()?;
                self.memory
                    .set_info(q, self.io.cur_tok as u16)
                    .expect("keyword token");
                if p == head {
                    self.memory.set_head_link(head, q).expect("keyword link");
                } else {
                    self.memory.set_link(p, q).expect("keyword link");
                }
                p = q;
                k += 1;
            } else if self.io.cur_cmd == cmd::SPACER && p == head {
                // Skip leading spaces before keyword
            } else {
                self.scanner.back_input(
                    &mut self.io,
                    &mut self.eqtb,
                    &mut self.hash,
                    &mut self.memory,
                )?;
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
                return Ok(false);
            }
        }
        let first = self.memory.head_link(head).unwrap_or(MemPtr::NULL);
        self.memory.flush_list(first)?;
        Ok(true)
    }

    /// §406: Skip optional spaces.
    pub fn get_x_token_skip_spaces(&mut self) -> Result<(), Termination> {
        loop {
            self.get_x_token()?;
            if self.io.cur_cmd != cmd::SPACER {
                break;
            }
        }
        Ok(())
    }

    /// §433: Scan an 8-bit integer (0..=255).
    pub fn scan_eight_bit_int(&mut self) -> Result<(), Termination> {
        self.scan_int()?;
        if self.scanner.cur_val < 0 || self.scanner.cur_val > 255 {
            self.io.print_err(pool_str("Bad register code"));
            self.scanner.cur_val = 0;
            self.error()?;
        }
        Ok(())
    }

    /// §434: Scan a character code (0..=255).
    pub fn scan_char_num(&mut self) -> Result<(), Termination> {
        self.scan_int()?;
        if self.scanner.cur_val < 0 || self.scanner.cur_val > 255 {
            self.io.print_err(pool_str("Bad character code"));
            self.scanner.cur_val = 0;
            self.error()?;
        }
        Ok(())
    }

    /// §435: Scan a 4-bit integer (0..=15).
    pub fn scan_four_bit_int(&mut self) -> Result<(), Termination> {
        self.scan_int()?;
        if self.scanner.cur_val < 0 || self.scanner.cur_val > 15 {
            self.io.print_err(pool_str("Bad number"));
            self.scanner.cur_val = 0;
            self.error()?;
        }
        Ok(())
    }

    /// §436: Scan a 15-bit integer (0..=32767).
    pub fn scan_fifteen_bit_int(&mut self) -> Result<(), Termination> {
        self.scan_int()?;
        if self.scanner.cur_val < 0 || self.scanner.cur_val > 0x7FFF {
            self.io.print_err(pool_str("Bad mathchar"));
            self.scanner.cur_val = 0;
            self.error()?;
        }
        Ok(())
    }

    /// §437: Scan a 27-bit integer (0..=0x7FF_FFFF).
    pub fn scan_twenty_seven_bit_int(&mut self) -> Result<(), Termination> {
        self.scan_int()?;
        if self.scanner.cur_val < 0 || self.scanner.cur_val > 0x07FF_FFFF {
            self.io.print_err(pool_str("Bad delimiter code"));
            self.scanner.cur_val = 0;
            self.error()?;
        }
        Ok(())
    }

    /// §§413–431: Scan an internal quantity.
    pub fn scan_something_internal(
        &mut self,
        level: u16,
        negative: bool,
    ) -> Result<(), Termination> {
        let m = self.io.cur_chr;
        match self.io.cur_cmd {
            cmd::DEF_CODE => {
                self.scan_char_num()?;
                let addr = m as usize + self.scanner.cur_val as usize;
                let mut val = i32::from(self.eqtb.equiv(addr));
                if m as usize == self.eqtb.layout.math_code_base && val == 0x8000 {
                    val = 0x8000;
                }
                self.scanner.cur_val = val;
                self.scanner.cur_val_level = INT_VAL;
            }
            cmd::TOKS_REGISTER | cmd::ASSIGN_TOKS => {
                let addr = if self.io.cur_cmd == cmd::TOKS_REGISTER {
                    self.scan_eight_bit_int()?;
                    self.eqtb.layout.toks_base + self.scanner.cur_val as usize
                } else {
                    m as usize
                };
                self.scanner.cur_val = i32::from(self.eqtb.equiv(addr));
                self.scanner.cur_val_level = TOK_VAL;
            }
            cmd::ASSIGN_INT => {
                self.scanner.cur_val = self.eqtb.int_par((m as usize) - self.eqtb.layout.int_base);
                self.scanner.cur_val_level = INT_VAL;
            }
            cmd::ASSIGN_DIMEN => {
                self.scanner.cur_val = self
                    .eqtb
                    .dimen_par((m as usize) - self.eqtb.layout.dimen_base)
                    .raw();
                self.scanner.cur_val_level = crate::tables::DIMEN_VAL;
            }
            cmd::ASSIGN_GLUE => {
                self.scanner.cur_val = i32::from(self.eqtb.equiv(m as usize));
                self.scanner.cur_val_level = crate::tables::GLUE_VAL;
            }
            cmd::ASSIGN_MU_GLUE => {
                self.scanner.cur_val = i32::from(self.eqtb.equiv(m as usize));
                self.scanner.cur_val_level = MU_VAL;
            }
            cmd::REGISTER => {
                self.scan_eight_bit_int()?;
                let idx = self.scanner.cur_val as usize;
                match m as u16 {
                    INT_VAL => {
                        self.scanner.cur_val =
                            i32::from(self.eqtb.equiv(self.eqtb.layout.count_base + idx));
                        self.scanner.cur_val_level = INT_VAL;
                    }
                    crate::tables::DIMEN_VAL => {
                        self.scanner.cur_val =
                            i32::from(self.eqtb.equiv(self.eqtb.layout.dimen_base + 256 + idx));
                        self.scanner.cur_val_level = crate::tables::DIMEN_VAL;
                    }
                    crate::tables::GLUE_VAL => {
                        self.scanner.cur_val =
                            i32::from(self.eqtb.equiv(self.eqtb.layout.skip_base + idx));
                        self.scanner.cur_val_level = crate::tables::GLUE_VAL;
                    }
                    MU_VAL => {
                        self.scanner.cur_val =
                            i32::from(self.eqtb.equiv(self.eqtb.layout.skip_base + 256 + idx));
                        self.scanner.cur_val_level = MU_VAL;
                    }
                    _ => {
                        self.scanner.cur_val = 0;
                        self.scanner.cur_val_level = INT_VAL;
                    }
                }
            }
            cmd::CHAR_GIVEN | cmd::MATH_GIVEN => {
                self.scanner.cur_val = m;
                self.scanner.cur_val_level = INT_VAL;
            }
            cmd::LAST_ITEM => {
                self.scanner.cur_val = 0;
                self.scanner.cur_val_level = if m == 0 {
                    INT_VAL
                } else {
                    crate::tables::DIMEN_VAL
                };
            }
            _ => {
                self.io.print_err(pool_str("You can't use `"));
                self.scanner.back_error(
                    &mut self.io,
                    &mut self.eqtb,
                    &mut self.hash,
                    &mut self.memory,
                )?;
                self.scanner.cur_val = 0;
                self.scanner.cur_val_level = level;
            }
        }

        // Convert between levels if needed (§§429–431)
        while self.scanner.cur_val_level > level {
            if self.scanner.cur_val_level == crate::tables::GLUE_VAL
                || self.scanner.cur_val_level == MU_VAL
            {
                let p = MemPtr::from_address(self.scanner.cur_val as u16);
                self.scanner.cur_val = self.memory.glue_spec(p).map(|s| s.width.raw()).unwrap_or(0);
            }
            self.scanner.cur_val_level -= 1;
        }

        if negative {
            if self.scanner.cur_val_level >= crate::tables::GLUE_VAL {
                let old = MemPtr::from_address(self.scanner.cur_val as u16);
                let new_p = self.memory.new_spec(old)?;
                if let Ok(mut spec) = self.memory.glue_spec(new_p) {
                    spec.width = Scaled::from_raw(-spec.width.raw()).unwrap_or(Scaled::ZERO);
                    spec.stretch = Scaled::from_raw(-spec.stretch.raw()).unwrap_or(Scaled::ZERO);
                    spec.shrink = Scaled::from_raw(-spec.shrink.raw()).unwrap_or(Scaled::ZERO);
                    self.memory.set_glue_spec(new_p, spec).ok();
                }
                self.scanner.cur_val = i32::from(new_p.address());
            } else {
                self.scanner.cur_val = -self.scanner.cur_val;
            }
        }
        Ok(())
    }

    /// §§440–447: Scan an integer.
    pub fn scan_int(&mut self) -> Result<(), Termination> {
        let mut negative = false;
        loop {
            self.get_x_token_skip_spaces()?;
            if self.io.cur_tok == i32::from(cmd::OTHER_CHAR) * 256 + b'-' as i32 {
                negative = !negative;
            } else if self.io.cur_tok != i32::from(cmd::OTHER_CHAR) * 256 + b'+' as i32 {
                break;
            }
        }

        if self.io.cur_tok == i32::from(cmd::OTHER_CHAR) * 256 + b'`' as i32 {
            // §442: Alphabetic constant
            self.get_token()?;
            if self.io.cur_tok < i32::from(CS_TOKEN_FLAG) {
                self.scanner.cur_val = self.io.cur_chr;
            } else if self.io.cur_tok
                < i32::from(CS_TOKEN_FLAG) + (self.eqtb.layout.single_base as i32)
            {
                self.scanner.cur_val = self.io.cur_tok
                    - i32::from(CS_TOKEN_FLAG)
                    - (self.eqtb.layout.active_base as i32);
            } else {
                self.scanner.cur_val = self.io.cur_tok
                    - i32::from(CS_TOKEN_FLAG)
                    - (self.eqtb.layout.single_base as i32);
            }
            if self.scanner.cur_val > 255 {
                self.io.print_err(pool_str("Improper alphabetic constant"));
                self.scanner.cur_val = b' ' as i32;
                self.error()?;
            } else {
                self.scan_optional_space()?;
            }
        } else if self.io.cur_cmd >= cmd::MIN_INTERNAL && self.io.cur_cmd <= cmd::MAX_INTERNAL {
            self.scan_something_internal(INT_VAL, false)?;
        } else {
            // §444: Octal, hex, or decimal digits
            self.scanner.radix = RADIX_DECIMAL;
            let mut m: i32 = 214_748_364; // INFINITY / 10
            if self.io.cur_tok == i32::from(cmd::OTHER_CHAR) * 256 + b'\'' as i32 {
                self.scanner.radix = RADIX_OCTAL;
                m = 0o1777777777; // 2^28 - 1
                self.get_x_token()?;
            } else if self.io.cur_tok == i32::from(cmd::OTHER_CHAR) * 256 + b'"' as i32 {
                self.scanner.radix = RADIX_HEX;
                m = 0x07FF_FFFF;
                self.get_x_token()?;
            }

            let mut vacuous = true;
            let mut val: i32 = 0;
            let mut ok_so_far = true;
            loop {
                let d: i32 = if self.io.cur_tok < i32::from(ZERO_TOKEN) + 10
                    && self.io.cur_tok >= i32::from(ZERO_TOKEN)
                    && self.io.cur_tok < i32::from(ZERO_TOKEN) + i32::from(self.scanner.radix)
                {
                    self.io.cur_tok - i32::from(ZERO_TOKEN)
                } else if self.scanner.radix == RADIX_HEX
                    && self.io.cur_tok >= i32::from(cmd::LETTER) * 256 + b'A' as i32
                    && self.io.cur_tok <= i32::from(cmd::LETTER) * 256 + b'F' as i32
                {
                    self.io.cur_tok - (i32::from(cmd::LETTER) * 256 + b'A' as i32) + 10
                } else if self.scanner.radix == RADIX_HEX
                    && self.io.cur_tok >= i32::from(cmd::OTHER_CHAR) * 256 + b'A' as i32
                    && self.io.cur_tok <= i32::from(cmd::OTHER_CHAR) * 256 + b'F' as i32
                {
                    self.io.cur_tok - (i32::from(cmd::OTHER_CHAR) * 256 + b'A' as i32) + 10
                } else {
                    break;
                };

                vacuous = false;
                if val >= m && (val > m || d > 7 || self.scanner.radix != RADIX_DECIMAL) {
                    if ok_so_far {
                        self.io.print_err(pool_str("Number too big"));
                        self.error()?;
                        val = INFINITY;
                        ok_so_far = false;
                    }
                } else {
                    val = val * i32::from(self.scanner.radix) + d;
                }
                self.get_x_token()?;
            }

            if vacuous {
                self.io
                    .print_err(pool_str("Missing number, treated as zero"));
                self.scanner.back_error(
                    &mut self.io,
                    &mut self.eqtb,
                    &mut self.hash,
                    &mut self.memory,
                )?;
            } else if self.io.cur_cmd != cmd::SPACER {
                self.scanner.back_input(
                    &mut self.io,
                    &mut self.eqtb,
                    &mut self.hash,
                    &mut self.memory,
                )?;
            }
            self.scanner.cur_val = val;
        }

        if negative {
            self.scanner.cur_val = -self.scanner.cur_val;
        }
        Ok(())
    }

    /// §443: Consume one optional space.
    pub fn scan_optional_space(&mut self) -> Result<(), Termination> {
        self.get_x_token()?;
        if self.io.cur_cmd != cmd::SPACER {
            self.scanner.back_input(
                &mut self.io,
                &mut self.eqtb,
                &mut self.hash,
                &mut self.memory,
            )?;
        }
        Ok(())
    }

    /// §§448–460: Scan a dimension (`scaled` integer).
    pub fn scan_dimen(&mut self, mu: bool, inf: bool, shortcut: bool) -> Result<(), Termination> {
        let mut negative = false;
        let mut f: i32 = 0;
        self.scanner.cur_order = NORMAL;

        if !shortcut {
            loop {
                self.get_x_token_skip_spaces()?;
                if self.io.cur_tok == i32::from(cmd::OTHER_CHAR) * 256 + b'-' as i32 {
                    negative = !negative;
                } else if self.io.cur_tok != i32::from(cmd::OTHER_CHAR) * 256 + b'+' as i32 {
                    break;
                }
            }

            if self.io.cur_cmd >= cmd::MIN_INTERNAL && self.io.cur_cmd <= cmd::MAX_INTERNAL {
                let target_level = if mu { MU_VAL } else { crate::tables::DIMEN_VAL };
                self.scan_something_internal(target_level, false)?;
                if self.scanner.cur_val_level >= crate::tables::DIMEN_VAL {
                    if negative {
                        self.scanner.cur_val = -self.scanner.cur_val;
                    }
                    return Ok(());
                }
            } else {
                self.scanner.back_input(
                    &mut self.io,
                    &mut self.eqtb,
                    &mut self.hash,
                    &mut self.memory,
                )?;
                let mut cur_tok_is_dot = false;
                self.get_x_token_skip_spaces()?;
                if self.io.cur_tok == i32::from(cmd::OTHER_CHAR) * 256 + b'.' as i32
                    || self.io.cur_tok == i32::from(cmd::OTHER_CHAR) * 256 + b',' as i32
                {
                    cur_tok_is_dot = true;
                    self.scanner.cur_val = 0;
                } else {
                    self.scanner.back_input(
                        &mut self.io,
                        &mut self.eqtb,
                        &mut self.hash,
                        &mut self.memory,
                    )?;
                    self.scan_int()?;
                    self.get_x_token()?;
                    if self.io.cur_tok == i32::from(cmd::OTHER_CHAR) * 256 + b'.' as i32
                        || self.io.cur_tok == i32::from(cmd::OTHER_CHAR) * 256 + b',' as i32
                    {
                        cur_tok_is_dot = true;
                    } else {
                        self.scanner.back_input(
                            &mut self.io,
                            &mut self.eqtb,
                            &mut self.hash,
                            &mut self.memory,
                        )?;
                    }
                }

                if cur_tok_is_dot {
                    let saved_val = self.scanner.cur_val;
                    let mut digits = [0u8; 18];
                    let mut k = 0usize;
                    loop {
                        self.get_x_token()?;
                        if self.io.cur_tok >= i32::from(ZERO_TOKEN)
                            && self.io.cur_tok <= i32::from(ZERO_TOKEN) + 9
                        {
                            if k < 17 {
                                digits[k] = (self.io.cur_tok - i32::from(ZERO_TOKEN)) as u8;
                                k += 1;
                            }
                        } else {
                            break;
                        }
                    }
                    if self.io.cur_cmd != cmd::SPACER {
                        self.scanner.back_input(
                            &mut self.io,
                            &mut self.eqtb,
                            &mut self.hash,
                            &mut self.memory,
                        )?;
                    }
                    f = round_decimals(&digits[..k]).map(Scaled::raw).unwrap_or(0);
                    self.scanner.cur_val = saved_val;
                }
            }
        }

        if self.scanner.cur_val < 0 {
            negative = !negative;
            self.scanner.cur_val = -self.scanner.cur_val;
        }

        // §454: Scan unit
        if inf && self.scan_keyword("fil")? {
            self.scanner.cur_order = FIL;
            while self.scan_keyword("l")? {
                if self.scanner.cur_order == FILLL {
                    self.io
                        .print_err(pool_str("Illegal unit of measure (replaced by filll)"));
                    self.error()?;
                } else {
                    self.scanner.cur_order += 1;
                }
            }
            self.scanner.cur_val = self.scanner.cur_val.saturating_mul(65536).saturating_add(f);
            self.scan_optional_space()?;
            if negative {
                self.scanner.cur_val = -self.scanner.cur_val;
            }
            return Ok(());
        }

        if mu {
            if !self.scan_keyword("mu")? {
                self.io
                    .print_err(pool_str("Illegal unit of measure (mu inserted)"));
                self.error()?;
            }
            self.scanner.cur_val = self.scanner.cur_val.saturating_mul(65536).saturating_add(f);
            self.scan_optional_space()?;
            if negative {
                self.scanner.cur_val = -self.scanner.cur_val;
            }
            return Ok(());
        }

        let _is_true = self.scan_keyword("true")?;
        let (num, den): (i32, i32) = if self.scan_keyword("pt")? {
            (1, 1)
        } else if self.scan_keyword("pc")? {
            (12, 1)
        } else if self.scan_keyword("in")? {
            (7227, 100)
        } else if self.scan_keyword("bp")? {
            (7227, 7200)
        } else if self.scan_keyword("cm")? {
            (7227, 254)
        } else if self.scan_keyword("mm")? {
            (7227, 2540)
        } else if self.scan_keyword("dd")? {
            (1238, 1157)
        } else if self.scan_keyword("cc")? {
            (14856, 1157)
        } else if self.scan_keyword("sp")? {
            self.scan_optional_space()?;
            if negative {
                self.scanner.cur_val = -self.scanner.cur_val;
            }
            return Ok(());
        } else if self.scan_keyword("em")? || self.scan_keyword("ex")? {
            // M5 will provide actual font quad/x_height; use 10pt / 4.3pt stub for M4.
            (10, 1)
        } else {
            self.io
                .print_err(pool_str("Illegal unit of measure (pt inserted)"));
            self.error()?;
            (1, 1)
        };

        let whole = self.scanner.cur_val;
        let scaled = if num == 1 && den == 1 {
            whole.saturating_mul(65536).saturating_add(f)
        } else {
            let total_sp =
                (i64::from(whole) * 65536 + i64::from(f)) * i64::from(num) / i64::from(den);
            total_sp.clamp(-i64::from(MAX_DIMEN), i64::from(MAX_DIMEN)) as i32
        };

        self.scanner.cur_val = scaled.clamp(-MAX_DIMEN, MAX_DIMEN);
        self.scan_optional_space()?;
        if negative {
            self.scanner.cur_val = -self.scanner.cur_val;
        }
        Ok(())
    }

    /// §§461–463: Scan a glue specification, returning a `GlueSpec` pointer in `cur_val`.
    pub fn scan_glue(&mut self, level: u16) -> Result<(), Termination> {
        let mu = level == MU_VAL;
        self.scan_dimen(mu, false, false)?;
        let width = self.scanner.cur_val;
        let mut stretch = 0;
        let mut stretch_order = NORMAL;
        let mut shrink = 0;
        let mut shrink_order = NORMAL;

        if self.scan_keyword("plus")? {
            self.scan_dimen(mu, true, false)?;
            stretch = self.scanner.cur_val;
            stretch_order = self.scanner.cur_order;
        }
        if self.scan_keyword("minus")? {
            self.scan_dimen(mu, true, false)?;
            shrink = self.scanner.cur_val;
            shrink_order = self.scanner.cur_order;
        }

        let crate::memory::NodeAllocation::Allocated(q) =
            self.memory.get_node(i32::from(GLUE_SPEC_SIZE))?
        else {
            return Err(Termination::JumpOut);
        };
        self.memory.set_glue_spec(
            q,
            crate::memory::GlueSpec {
                ref_count: 0,
                width: Scaled::from_raw(width).unwrap_or(Scaled::ZERO),
                stretch: Scaled::from_raw(stretch).unwrap_or(Scaled::ZERO),
                shrink: Scaled::from_raw(shrink).unwrap_or(Scaled::ZERO),
                stretch_order,
                shrink_order,
            },
        )?;
        self.scanner.cur_val = i32::from(q.address());
        Ok(())
    }

    /// §§473–481: Scan a token list (`macro_def` for `\def`/`\edef`, `xpand` for `\edef`/`\xdef`).
    /// Returns the head `MemPtr` of the reference-counted token list.
    pub fn scan_toks(&mut self, macro_def: bool, xpand: bool) -> Result<MemPtr, Termination> {
        let saved_status = self.scanner.scanner_status;
        if macro_def {
            self.scanner.scanner_status = ScannerStatus::Defining;
        } else {
            self.scanner.scanner_status = ScannerStatus::Absorbing;
        }

        let p = self.memory.get_avail()?;
        self.memory.set_info(p, 0).unwrap(); // ref_count = 0 (will be set by caller or 0)
        self.memory.set_link(p, MemPtr::NULL).unwrap();
        self.scanner.def_ref = p;
        let mut tail = p;
        let mut hash_brace: u16 = 0;
        let mut t: u16 = 0;

        if macro_def {
            // §474: Scan parameter text
            loop {
                self.get_token()?;
                if self.io.cur_tok < i32::from(RIGHT_BRACE_LIMIT) {
                    break;
                }
                if self.io.cur_cmd == cmd::MAC_PARAM {
                    // Parameter #1..#9 or #{
                    self.get_token()?;
                    if self.io.cur_tok < i32::from(LEFT_BRACE_LIMIT) {
                        hash_brace = self.io.cur_tok as u16;
                        let q = self.memory.get_avail()?;
                        self.memory.set_info(q, MATCH_TOKEN + t).unwrap();
                        self.memory.set_link(tail, q).unwrap();
                        tail = q;
                        let q2 = self.memory.get_avail()?;
                        self.memory.set_info(q2, hash_brace).unwrap();
                        self.memory.set_link(tail, q2).unwrap();
                        tail = q2;
                        t += 1;
                        break;
                    }
                    if t >= 9 {
                        self.io
                            .print_err(pool_str("You already have nine parameters"));
                        self.error()?;
                    } else {
                        t += 1;
                        let expected = i32::from(ZERO_TOKEN) + i32::from(t);
                        if self.io.cur_tok != expected {
                            self.io
                                .print_err(pool_str("Parameters must be numbered consecutively"));
                            self.scanner.back_error(
                                &mut self.io,
                                &mut self.eqtb,
                                &mut self.hash,
                                &mut self.memory,
                            )?;
                        }
                        let q = self.memory.get_avail()?;
                        self.memory.set_info(q, MATCH_TOKEN + (t - 1)).unwrap();
                        self.memory.set_link(tail, q).unwrap();
                        tail = q;
                    }
                } else {
                    let q = self.memory.get_avail()?;
                    self.memory.set_info(q, self.io.cur_tok as u16).unwrap();
                    self.memory.set_link(tail, q).unwrap();
                    tail = q;
                }
            }
            let end_q = self.memory.get_avail()?;
            self.memory.set_info(end_q, END_MATCH_TOKEN).unwrap();
            self.memory.set_link(tail, end_q).unwrap();
            tail = end_q;
            if self.io.cur_cmd == cmd::RIGHT_BRACE {
                self.io.print_err(pool_str("Missing { inserted"));
                self.io.align_state += 1;
                self.error()?;
            }
        } else {
            self.scan_left_brace()?;
        }

        // §477: Scan replacement text
        let mut unbalance: i32 = 1;
        loop {
            if xpand {
                self.get_next()?;
                if self.io.cur_cmd > cmd::MAX_COMMAND {
                    if self.io.cur_cmd == cmd::THE {
                        let q = self.the_toks()?;
                        if q != MemPtr::NULL {
                            self.memory.set_link(tail, q).unwrap();
                            while let Ok(nxt) = self.memory.link(tail) {
                                if nxt == MemPtr::NULL {
                                    break;
                                }
                                tail = nxt;
                            }
                        }
                        continue;
                    }
                    self.expand()?;
                    continue;
                }
            } else {
                self.get_token()?;
            }

            if self.io.cur_tok < i32::from(RIGHT_BRACE_LIMIT) {
                if self.io.cur_cmd == cmd::LEFT_BRACE {
                    unbalance += 1;
                } else {
                    unbalance -= 1;
                    if unbalance == 0 {
                        break;
                    }
                }
            } else if self.io.cur_cmd == cmd::MAC_PARAM && macro_def {
                self.get_token()?;
                if self.io.cur_cmd == cmd::MAC_PARAM {
                    // ## -> single #
                } else if self.io.cur_tok > i32::from(ZERO_TOKEN)
                    && self.io.cur_tok <= i32::from(ZERO_TOKEN) + i32::from(t)
                {
                    let param_idx = (self.io.cur_tok - i32::from(ZERO_TOKEN) - 1) as u16;
                    let q = self.memory.get_avail()?;
                    self.memory
                        .set_info(q, OUT_PARAM_TOKEN + param_idx)
                        .unwrap();
                    self.memory.set_link(tail, q).unwrap();
                    tail = q;
                    continue;
                } else {
                    self.io
                        .print_err(pool_str("Illegal parameter number in definition"));
                    self.scanner.back_error(
                        &mut self.io,
                        &mut self.eqtb,
                        &mut self.hash,
                        &mut self.memory,
                    )?;
                    continue;
                }
            }

            let q = self.memory.get_avail()?;
            self.memory.set_info(q, self.io.cur_tok as u16).unwrap();
            self.memory.set_link(tail, q).unwrap();
            tail = q;
        }

        if hash_brace != 0 {
            let q = self.memory.get_avail()?;
            self.memory.set_info(q, hash_brace).unwrap();
            self.memory.set_link(tail, q).unwrap();
        }

        self.scanner.scanner_status = saved_status;
        self.scanner.def_ref = MemPtr::NULL;
        Ok(p)
    }

    /// §465: Build token list for `\the`, returning the first token node (or `MemPtr::NULL`).
    pub fn the_toks(&mut self) -> Result<MemPtr, Termination> {
        self.get_x_token()?;
        self.scan_something_internal(TOK_VAL, false)?;
        let head = self.memory.temp_head();
        self.memory.set_head_link(head, MemPtr::NULL).unwrap();
        let mut tail = head;

        match self.scanner.cur_val_level {
            INT_VAL => {
                let s = self.scanner.cur_val.to_string();
                for &b in s.as_bytes() {
                    let q = self.memory.get_avail()?;
                    self.memory
                        .set_info(q, cmd::OTHER_CHAR as u16 * 256 + u16::from(b))
                        .unwrap();
                    self.memory.set_link(tail, q).unwrap();
                    tail = q;
                }
            }
            crate::tables::DIMEN_VAL => {
                let s = format!("{}sp", self.scanner.cur_val);
                for &b in s.as_bytes() {
                    let q = self.memory.get_avail()?;
                    let c = if b.is_ascii_alphabetic() {
                        cmd::LETTER as u16 * 256 + u16::from(b)
                    } else {
                        cmd::OTHER_CHAR as u16 * 256 + u16::from(b)
                    };
                    self.memory.set_info(q, c).unwrap();
                    self.memory.set_link(tail, q).unwrap();
                    tail = q;
                }
            }
            TOK_VAL => {
                let mut p = MemPtr::from_address(self.scanner.cur_val as u16);
                if p != MemPtr::NULL {
                    p = self.memory.link(p).unwrap_or(MemPtr::NULL);
                    while p != MemPtr::NULL {
                        let q = self.memory.get_avail()?;
                        self.memory
                            .set_info(q, self.memory.info(p).unwrap())
                            .unwrap();
                        self.memory.set_link(tail, q).unwrap();
                        tail = q;
                        p = self.memory.link(p).unwrap_or(MemPtr::NULL);
                    }
                }
            }
            _ => {}
        }
        Ok(self.memory.link(head).unwrap_or(MemPtr::NULL))
    }

    /// §467: Insert `\the` tokens into the input stream.
    pub fn ins_the_toks(&mut self) -> Result<(), Termination> {
        let first = self.the_toks()?;
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

    /// §526: Token-driven `scan_file_name`.
    pub fn scan_file_name(&mut self) -> Result<(), Termination> {
        self.io.begin_name();
        self.io.name_in_progress = true;
        loop {
            self.get_x_token()?;
            if self.io.cur_cmd != cmd::SPACER {
                break;
            }
        }
        loop {
            if self.scanner.cur_cs != 0 || self.io.cur_cmd == cmd::SPACER {
                self.scanner.back_input(
                    &mut self.io,
                    &mut self.eqtb,
                    &mut self.hash,
                    &mut self.memory,
                )?;
                break;
            }
            let c = self.io.cur_chr as u8;
            if !self
                .io
                .more_name_with_hooks(c, &mut crate::io::NoopErrorHooks)?
            {
                self.scanner.back_input(
                    &mut self.io,
                    &mut self.eqtb,
                    &mut self.hash,
                    &mut self.memory,
                )?;
                break;
            }
            self.get_x_token()?;
        }
        self.io
            .end_name_with_hooks(&mut crate::io::NoopErrorHooks)?;
        self.io.name_in_progress = false;
        Ok(())
    }
}
