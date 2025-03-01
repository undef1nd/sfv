use crate::GenericBareItem;
use std::convert::{TryFrom, TryInto};
use std::fmt;

const RANGE_I64: std::ops::RangeInclusive<i64> = -999_999_999_999_999..=999_999_999_999_999;

/// A structured field value [integer].
///
/// [integer]: <https://httpwg.org/specs/rfc8941.html#integer>
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Integer(
    #[cfg_attr(
        feature = "arbitrary",
        arbitrary(with = |u: &mut arbitrary::Unstructured| u.int_in_range(RANGE_I64))
    )]
    i64,
);

impl Integer {
    /// The minimum value for a parsed or serialized integer: `-999_999_999_999_999`.
    pub const MIN: Self = Self(*RANGE_I64.start());

    /// The maximum value for a parsed or serialized integer: `999_999_999_999_999`.
    pub const MAX: Self = Self(*RANGE_I64.end());

    /// `0`.
    ///
    /// Equivalent to `Integer::constant(0)`.
    pub const ZERO: Self = Self(0);

    /// Creates an `Integer`, panicking if the value is out of range.
    ///
    /// This method is intended to be called from `const` contexts in which the
    /// value is known to be valid. Use [`TryFrom::try_from`] for non-panicking
    /// conversions.
    pub const fn constant(v: i64) -> Self {
        if v >= Self::MIN.0 && v <= Self::MAX.0 {
            Self(v)
        } else {
            panic!("out of range for Integer")
        }
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// An error that occurs when a value is out of range.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct OutOfRangeError;

impl fmt::Display for OutOfRangeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("out of range")
    }
}

impl std::error::Error for OutOfRangeError {}

macro_rules! impl_conversions {
    ($($t: ty: $from:ident => $into:ident,)+) => {
        $(
            impl_conversion!($from<$t>);
            impl_conversion!($into<$t>);
        )+
    }
}

macro_rules! impl_conversion {
    (From<$t: ty>) => {
        impl From<$t> for Integer {
            fn from(v: $t) -> Integer {
                Integer(v.into())
            }
        }
        impl<S, B, T> From<$t> for GenericBareItem<S, B, T> {
            fn from(v: $t) -> Self {
                Self::Integer(v.into())
            }
        }
    };
    (TryFrom<$t: ty>) => {
        impl TryFrom<$t> for Integer {
            type Error = OutOfRangeError;

            fn try_from(v: $t) -> Result<Integer, OutOfRangeError> {
                match i64::try_from(v) {
                    Ok(v) if RANGE_I64.contains(&v) => Ok(Integer(v)),
                    _ => Err(OutOfRangeError),
                }
            }
        }
        impl<S, B, T> TryFrom<$t> for GenericBareItem<S, B, T> {
            type Error = OutOfRangeError;

            fn try_from(v: $t) -> Result<Self, OutOfRangeError> {
                Integer::try_from(v).map(Self::Integer)
            }
        }
    };
    (Into<$t: ty>) => {
        impl From<Integer> for $t {
            fn from(v: Integer) -> $t {
                v.0.into()
            }
        }
    };
    (TryInto<$t: ty>) => {
        impl TryFrom<Integer> for $t {
            type Error = OutOfRangeError;

            fn try_from(v: Integer) -> Result<$t, OutOfRangeError> {
                v.0.try_into().map_err(|_| OutOfRangeError)
            }
        }
    };
}

impl_conversions! {
    i8: From => TryInto,
    i16: From => TryInto,
    i32: From => TryInto,
    i64: TryFrom => Into,
    i128: TryFrom => Into,
    isize: TryFrom => TryInto,

    u8: From => TryInto,
    u16: From => TryInto,
    u32: From => TryInto,
    u64: TryFrom => TryInto,
    u128: TryFrom => TryInto,
    usize: TryFrom => TryInto,
}

/// Creates an `Integer`, panicking if the value is out of range.
///
/// This is a convenience free function for [`Integer::constant`].
///
/// This method is intended to be called from `const` contexts in which the
/// value is known to be valid. Use [`TryFrom::try_from`] for non-panicking
/// conversions.
pub const fn integer(v: i64) -> Integer {
    Integer::constant(v)
}
