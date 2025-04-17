use crate::{Decimal, Error, Integer};

#[test]
fn test_display() {
    for (expected, input) in [
        ("0.1", 100),
        ("-0.1", -100),
        ("0.01", 10),
        ("-0.01", -10),
        ("0.001", 1),
        ("-0.001", -1),
        ("0.12", 120),
        ("-0.12", -120),
        ("0.124", 124),
        ("-0.124", -124),
        ("0.125", 125),
        ("-0.125", -125),
        ("0.126", 126),
        ("-0.126", -126),
    ] {
        let decimal = Decimal::from_integer_scaled_1000(Integer::constant(input));
        assert_eq!(expected, decimal.to_string());
    }

    assert_eq!("0.0", Decimal::ZERO.to_string());
    assert_eq!("-999999999999.999", Decimal::MIN.to_string());
    assert_eq!("999999999999.999", Decimal::MAX.to_string());
}

#[test]
fn test_into_f64() {
    for (expected, input) in [
        (0.0, 0),
        (0.001, 1),
        (0.01, 10),
        (0.1, 100),
        (1.0, 1000),
        (10.0, 10000),
        (0.123, 123),
        (-0.001, -1),
        (-0.01, -10),
        (-0.1, -100),
        (-1.0, -1000),
        (-10.0, -10000),
        (-0.123, -123),
    ] {
        assert_eq!(
            expected,
            f64::from(Decimal::from_integer_scaled_1000(input.into()))
        );
    }

    assert_eq!(-999_999_999_999.999, f64::from(Decimal::MIN));

    assert_eq!(999_999_999_999.999, f64::from(Decimal::MAX));
}

#[test]
fn test_try_from_f64() {
    for (expected, input) in [
        (Err(Error::new("NaN")), f64::NAN),
        (Err(Error::out_of_range()), f64::INFINITY),
        (Err(Error::out_of_range()), f64::NEG_INFINITY),
        (Err(Error::out_of_range()), -1_000_000_000_000.0),
        (Err(Error::out_of_range()), 1_000_000_000_000.0),
        (Ok(Decimal::MIN), -999_999_999_999.999),
        (Ok(Decimal::MIN), -999_999_999_999.999_1),
        (Err(Error::out_of_range()), -999_999_999_999.999_5),
        (Ok(Decimal::MAX), 999_999_999_999.999),
        (Ok(Decimal::MAX), 999_999_999_999.999_1),
        (Err(Error::out_of_range()), 999_999_999_999.999_5),
        (Ok(Decimal::ZERO), 0.0),
        (Ok(Decimal::ZERO), -0.0),
        (
            Ok(Decimal::from_integer_scaled_1000(Integer::constant(123))),
            0.1234,
        ),
        (
            Ok(Decimal::from_integer_scaled_1000(Integer::constant(124))),
            0.1235,
        ),
        (
            Ok(Decimal::from_integer_scaled_1000(Integer::constant(124))),
            0.1236,
        ),
        (
            Ok(Decimal::from_integer_scaled_1000(Integer::constant(-123))),
            -0.1234,
        ),
        (
            Ok(Decimal::from_integer_scaled_1000(Integer::constant(-124))),
            -0.1235,
        ),
        (
            Ok(Decimal::from_integer_scaled_1000(Integer::constant(-124))),
            -0.1236,
        ),
    ] {
        assert_eq!(expected, Decimal::try_from(input));
    }
}
