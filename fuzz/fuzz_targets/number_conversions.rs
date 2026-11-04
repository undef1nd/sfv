#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(arbitrary::Arbitrary, Debug)]
enum Input {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    F64(f64),
}

fn check_infallible<T>(v: T)
where
    T: Copy
        + Eq
        + std::fmt::Debug
        + Into<sfv::Integer>
        + Into<sfv::Decimal>
        + TryFrom<sfv::Integer, Error = sfv::Error>,
{
    let int: sfv::Integer = v.into();
    assert_eq!(T::try_from(int).unwrap(), v);

    let _: sfv::Decimal = v.into();
}

fn check_fallible<T>(v: T)
where
    T: Copy + Eq + std::fmt::Debug + TryFrom<sfv::Integer, Error: std::fmt::Debug>,
    sfv::Integer: TryFrom<T, Error = sfv::Error>,
    sfv::Decimal: TryFrom<T, Error = sfv::Error>,
{
    if let Ok(int) = sfv::Integer::try_from(v) {
        assert_eq!(T::try_from(int).unwrap(), v);
    }

    let _ = sfv::Decimal::try_from(v);
}

fuzz_target!(|input: Input| {
    match input {
        Input::I8(v) => check_infallible(v),
        Input::I16(v) => check_infallible(v),
        Input::I32(v) => check_infallible(v),
        Input::I64(v) => check_fallible(v),
        Input::I128(v) => check_fallible(v),
        Input::Isize(v) => check_fallible(v),
        Input::U8(v) => check_infallible(v),
        Input::U16(v) => check_infallible(v),
        Input::U32(v) => check_infallible(v),
        Input::U64(v) => check_fallible(v),
        Input::U128(v) => check_fallible(v),
        Input::Usize(v) => check_fallible(v),
        Input::F64(v) => {
            if let Ok(d) = sfv::Decimal::try_from(v) {
                assert_eq!(sfv::Decimal::try_from(f64::from(d)).unwrap(), d);
            }
        }
    }
});
