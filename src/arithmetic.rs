//! Knuth, tex.web §§99–108. Detailed domains and bounds: docs/semantics.md.

/// Internal scaled integer (§101), not merely an input dimension (§104).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Scaled(i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreconditionError {
    IntegerRange,
    MultAndAddDomain,
    FractionDomain,
    IntermediateRange,
    NegativeBadnessAmount,
}

impl Scaled {
    pub const UNITY: Self = Self(65536);
    pub const ZERO: Self = Self(0);

    pub fn new(value: i32) -> Result<Self, PreconditionError> {
        if value == i32::MIN {
            Err(PreconditionError::IntegerRange)
        } else {
            Ok(Self(value))
        }
    }

    pub const fn raw(self) -> i32 {
        self.0
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arithmetic {
    arith_error: bool,
    remainder: Scaled,
}

impl Arithmetic {
    pub fn error(&self) -> bool {
        self.arith_error
    }

    pub fn remainder(&self) -> Scaled {
        self.remainder
    }

    /// A caller reset, never implicitly performed by arithmetic routines (§104).
    pub fn clear_error(&mut self) {
        self.arith_error = false;
    }

    /// §105: n*x+y with the source's two overflow guards.
    pub fn mult_and_add(
        &mut self,
        n: i32,
        x: Scaled,
        y: Scaled,
        max_answer: i32,
    ) -> Result<Scaled, PreconditionError> {
        Scaled::new(n)?;
        if max_answer < 0 {
            return Err(PreconditionError::MultAndAddDomain);
        }
        let (mut n, mut x, y, max) = (
            i64::from(n),
            i64::from(x.0),
            i64::from(y.0),
            i64::from(max_answer),
        );
        if n == 0 {
            return scaled_result(y);
        }
        if y.abs() > max || max + y.abs() > i64::from(i32::MAX) {
            return Err(PreconditionError::MultAndAddDomain);
        }
        if n < 0 {
            x = -x;
            n = -n;
        }
        if x <= (max - y) / n && -x <= (max + y) / n {
            scaled_result(n * x + y)
        } else {
            self.arith_error = true;
            Ok(Scaled::ZERO)
        }
    }

    /// §106: truncation toward zero; remainder always has the dividend's sign.
    pub fn x_over_n(&mut self, x: Scaled, n: i32) -> Result<Scaled, PreconditionError> {
        Scaled::new(n)?;
        if n == 0 {
            self.arith_error = true;
            self.remainder = x;
            return Ok(Scaled::ZERO);
        }
        let mut x = i64::from(x.0);
        let mut n = i64::from(n);
        let negative = n < 0;
        if negative {
            x = -x;
            n = -n;
        }
        let (q, mut r) = if x >= 0 {
            (x / n, x % n)
        } else {
            (-((-x) / n), -((-x) % n))
        };
        if negative {
            r = -r;
        }
        let q = scaled_result(q)?;
        self.remainder = scaled_result(r)?;
        Ok(q)
    }

    /// §107: base-32768 multiplication/division, including its error result.
    pub fn xn_over_d(&mut self, x: Scaled, n: i32, d: i32) -> Result<Scaled, PreconditionError> {
        if !(0..=65536).contains(&n) || !(1..=65536).contains(&d) {
            return Err(PreconditionError::FractionDomain);
        }
        let positive = x.0 >= 0;
        let x = i64::from(x.0).abs();
        let (n, d) = (i64::from(n), i64::from(d));
        let t = (x % 32768) * n;
        let mut u = (x / 32768) * n + t / 32768;
        let v = (u % d) * 32768 + t % 32768;
        if [t, u, v].iter().any(|&z| z > i64::from(i32::MAX)) {
            return Err(PreconditionError::IntermediateRange);
        }
        let overflow = u / d >= 32768;
        if !overflow {
            u = 32768 * (u / d) + v / d;
        }
        let sign = if positive { 1 } else { -1 };
        let answer = scaled_result(sign * u)?;
        let remainder = scaled_result(sign * (v % d))?;
        self.arith_error |= overflow;
        self.remainder = remainder;
        Ok(answer)
    }
}

fn scaled_result(value: i64) -> Result<Scaled, PreconditionError> {
    Scaled::new(i32::try_from(value).map_err(|_| PreconditionError::IntermediateRange)?)
}

/// §108: the specified integer heuristic, not a floating-point approximation.
pub fn badness(t: Scaled, s: Scaled) -> Result<u16, PreconditionError> {
    let (t, s) = (i64::from(t.0), i64::from(s.0));
    if t < 0 {
        return Err(PreconditionError::NegativeBadnessAmount);
    }
    if t == 0 {
        return Ok(0);
    }
    if s <= 0 {
        return Ok(10000);
    }
    let r = if t <= 7_230_584 {
        (t * 297) / s
    } else if s >= 1_663_497 {
        t / (s / 297)
    } else {
        t
    };
    if r > 1290 {
        Ok(10000)
    } else {
        u16::try_from((r * r * r + 131072) / 262144)
            .map_err(|_| PreconditionError::IntermediateRange)
    }
}
