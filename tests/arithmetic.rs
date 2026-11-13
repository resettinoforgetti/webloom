use webloom::arithmetic::{Arithmetic, PreconditionError, Scaled, badness};

fn s(x: i32) -> Scaled {
    Scaled::new(x).unwrap()
}

#[test]
fn scaled_is_an_internal_integer_not_just_an_input_dimension() {
    assert!(Scaled::new(i32::MIN).is_err());
    assert_eq!(s(i32::MAX).raw(), i32::MAX);
    assert_eq!(s(-i32::MAX).raw(), -i32::MAX);
    assert_eq!(Scaled::UNITY.raw(), 65536);
}

#[test]
fn mult_and_add_boundaries_and_sticky_state() {
    let mut a = Arithmetic::default();
    a.x_over_n(s(17), 5).unwrap();
    for (n, x, y, max, answer, error) in [
        (0, i32::MAX, 99, 0, 99, false),
        (3, 10, -2, 28, 28, false),
        (-3, 10, 2, 28, -28, false),
        (3, 10, -2, 27, 0, true),
        (-3, 10, 2, 27, 0, true),
        (1, i32::MAX, 0, i32::MAX, i32::MAX, false),
        (2, 1073741824, 0, i32::MAX, 0, true),
        (i32::MAX, 0, 0, i32::MAX, 0, false),
        (-i32::MAX, -1, 0, i32::MAX, i32::MAX, false),
    ] {
        a.clear_error();
        assert_eq!(a.mult_and_add(n, s(x), s(y), max).unwrap().raw(), answer);
        assert_eq!(a.error(), error);
        assert_eq!(a.remainder().raw(), 2);
    }
    a.mult_and_add(2, s(i32::MAX), s(0), i32::MAX).unwrap();
    a.mult_and_add(1, s(1), s(0), 1).unwrap();
    assert!(a.error());
}

#[test]
fn signed_division_zero_ties_and_remainder_identity() {
    let mut a = Arithmetic::default();
    for x in [-i32::MAX, -11, -5, -1, 0, 1, 5, 11, i32::MAX] {
        for n in [-i32::MAX, -5, -2, -1, 1, 2, 5, i32::MAX] {
            let q = a.x_over_n(s(x), n).unwrap().raw();
            let r = a.remainder().raw();
            assert_eq!(i64::from(q), i64::from(x) / i64::from(n));
            assert_eq!(i64::from(x), i64::from(q) * i64::from(n) + i64::from(r));
            assert!(r.abs() < n.abs());
            assert!(r == 0 || r.signum() == x.signum());
        }
    }
    assert_eq!(a.x_over_n(s(-7), 0).unwrap().raw(), 0);
    assert_eq!(a.remainder().raw(), -7);
    assert!(a.error());
    assert_eq!(a.x_over_n(s(5), 2).unwrap().raw(), 2);
    assert!(a.error());
    a.clear_error();
    assert!(!a.error());
    assert_eq!(a.remainder().raw(), 1);
}

#[test]
fn fraction_errors_preserve_source_return_and_remainder() {
    let mut a = Arithmetic::default();
    for sign in [-1, 1] {
        // Exact product is 2^30+1, overflow result is floor(product/32768).
        assert_eq!(
            a.xn_over_d(s(sign * 1073741825), 1, 1).unwrap().raw(),
            sign * 32768
        );
        assert!(a.error());
        assert_eq!(a.remainder().raw(), 0);
        a.clear_error();
        assert_eq!(
            a.xn_over_d(s(sign * 1073741825), 3, 2).unwrap().raw(),
            sign * 98304
        );
        assert_eq!(a.remainder().raw(), sign);
        assert!(a.error());
    }
    assert_eq!(a.xn_over_d(s(-5), 1, 2).unwrap().raw(), -2);
    assert_eq!(a.remainder().raw(), -1);
    assert!(a.error());
    a.clear_error();
    assert_eq!(a.xn_over_d(s(i32::MAX), 0, 65536).unwrap().raw(), 0);
    assert!(!a.error());
    assert_eq!(
        a.xn_over_d(s(1073741823), 65536, 65536).unwrap().raw(),
        1073741823
    );
    assert!(!a.error());
}

#[test]
fn invalid_preconditions_are_not_tex_arithmetic_errors() {
    let mut a = Arithmetic::default();
    a.x_over_n(s(-13), 0).unwrap();
    let before = a.clone();
    assert_eq!(
        a.mult_and_add(i32::MIN, s(1), s(0), 1),
        Err(PreconditionError::IntegerRange)
    );
    assert!(a.mult_and_add(1, s(1), s(2), 1).is_err());
    assert!(a.mult_and_add(1, s(0), s(1), i32::MAX).is_err());
    assert!(a.mult_and_add(0, s(0), s(0), -1).is_err());
    assert!(a.x_over_n(s(1), i32::MIN).is_err());
    for (n, d) in [(-1, 1), (65537, 1), (1, 0), (1, -1), (1, 65537)] {
        assert_eq!(
            a.xn_over_d(s(1), n, d),
            Err(PreconditionError::FractionDomain)
        );
    }
    assert_eq!(
        a.xn_over_d(s(i32::MAX), 65536, 1),
        Err(PreconditionError::IntermediateRange)
    );
    assert_eq!(a, before);
    assert!(badness(s(-1), s(1)).is_err());
}

// Deterministic data generation only; wrapping here has no TeX semantics.
fn next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    *seed
}

#[test]
fn wide_product_oracle_for_arithmetic_sequences() {
    let mut seed = 0x82;
    let mut a = Arithmetic::default();
    for _ in 0..20000 {
        let x = (next(&mut seed) % 2147483647) as i64 - 1073741823;
        let n = (next(&mut seed) % 65537) as i64;
        let d = (next(&mut seed) % 65536 + 1) as i64;
        // x is bounded by 2^30-1, so all §107 intermediates fit for n<=65536.
        let product = x * n;
        let error = product.abs() / d >= 1073741824;
        let expected = if error { product / 32768 } else { product / d };
        a.clear_error();
        assert_eq!(
            i64::from(a.xn_over_d(s(x as i32), n as i32, d as i32).unwrap().raw()),
            expected
        );
        assert_eq!(i64::from(a.remainder().raw()), product % d);
        assert_eq!(a.error(), error);

        let n = n - 32768;
        let y = (next(&mut seed) % 100001) as i64 - 50000;
        let max = 1073741823_i64;
        let exact = n * x + y;
        a.clear_error();
        let answer = a
            .mult_and_add(n as i32, s(x as i32), s(y as i32), max as i32)
            .unwrap();
        assert_eq!(
            i64::from(answer.raw()),
            if exact.abs() > max { 0 } else { exact }
        );
        assert_eq!(a.error(), exact.abs() > max);
    }
}

#[test]
fn badness_exact_vectors_and_all_reachable_cube_roundings() {
    for (t, supply, answer) in [
        (0, 0, 0),
        (0, -1, 0),
        (1, 0, 10000),
        (1, -1, 10000),
        (1, 1, 100),
        (1, 2, 12),
        (1, 3, 4),
        (4, 1, 6396),
        (1290, 297, 8189),
        (1291, 297, 10000),
        (7230584, 1663497, 8189),
        (7230585, 1663497, 8189),
        (7230585, 1663496, 10000),
        (i32::MAX, i32::MAX, 100),
        (7230584, i32::MAX, 0),
    ] {
        assert_eq!(
            badness(s(t), s(supply)).unwrap(),
            answer,
            "t={t}, s={supply}"
        );
    }
    for r in 0..=1290_i32 {
        // With s=297, the quantized ratio is exactly r. An independent
        // nearest-integer oracle uses quotient/remainder, not the rounding sum.
        let cube = i64::from(r).pow(3);
        let floor = cube / 262144;
        let rounded = floor + i64::from(cube % 262144 >= 131072);
        assert_eq!(i64::from(badness(s(r), s(297)).unwrap()), rounded);
    }
}
