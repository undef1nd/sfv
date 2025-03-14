use crate::{Error, Integer};

use std::convert::{TryFrom, TryInto};
use std::fmt;

/// A structured field value [decimal].
///
/// Decimals have 12 digits of integer precision and 3 digits of fractional precision.
///
/// [decimal]: <https://httpwg.org/specs/rfc8941.html#decimal>
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Decimal(Integer);

impl Decimal {
    /// The minimum value for a parsed or serialized decimal: `-999_999_999_999.999`.
    pub const MIN: Self = Self(Integer::MIN);

    /// The maximum value for a parsed or serialized decimal: `999_999_999_999.999`.
    pub const MAX: Self = Self(Integer::MAX);

    /// `0.0`.
    pub const ZERO: Self = Self(Integer::ZERO);

    /// Returns the decimal as an integer multiplied by 1000.
    ///
    /// The conversion is guaranteed to be precise.
    ///
    /// # Example
    ///
    /// ```
    /// use std::convert::TryFrom;
    ///
    /// let decimal = sfv::Decimal::try_from(1.234).unwrap();
    /// assert_eq!(i64::from(decimal.as_integer_scaled_1000()), 1234);
    /// ````
    pub fn as_integer_scaled_1000(&self) -> Integer {
        self.0
    }

    /// Creates a decimal from an integer multiplied by 1000.
    ///
    /// The conversion is guaranteed to be precise.
    ///
    /// # Example
    ///
    /// ```
    /// let decimal = sfv::Decimal::from_integer_scaled_1000(sfv::integer(1234));
    /// assert_eq!(f64::from(decimal), 1.234);
    /// ````
    pub const fn from_integer_scaled_1000(v: Integer) -> Self {
        Self(v)
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v = i64::from(self.as_integer_scaled_1000());

        if v == 0 {
            return f.write_str("0.0");
        }

        let sign = if v < 0 { "-" } else { "" };
        let v = v.abs();
        let i_part = v / 1000;
        let f_part = v % 1000;

        if f_part % 100 == 0 {
            write!(f, "{}{}.{}", sign, i_part, f_part / 100)
        } else if f_part % 10 == 0 {
            write!(f, "{}{}.{:02}", sign, i_part, f_part / 10)
        } else {
            write!(f, "{}{}.{:03}", sign, i_part, f_part)
        }
    }
}

impl From<i8> for Decimal {
    fn from(v: i8) -> Decimal {
        Self(Integer::from(v as i16 * 1000))
    }
}

impl From<i16> for Decimal {
    fn from(v: i16) -> Decimal {
        Self(Integer::from(v as i32 * 1000))
    }
}

impl From<i32> for Decimal {
    fn from(v: i32) -> Decimal {
        Self(Integer::try_from(v as i64 * 1000).unwrap())
    }
}

impl TryFrom<i64> for Decimal {
    type Error = Error;

    fn try_from(v: i64) -> Result<Decimal, Error> {
        match v.checked_mul(1000) {
            None => Err(Error::out_of_range()),
            Some(v) => match Integer::try_from(v) {
                Ok(v) => Ok(Decimal(v)),
                Err(_) => Err(Error::out_of_range()),
            },
        }
    }
}

impl TryFrom<i128> for Decimal {
    type Error = Error;

    fn try_from(v: i128) -> Result<Decimal, Error> {
        match v.checked_mul(1000) {
            None => Err(Error::out_of_range()),
            Some(v) => match Integer::try_from(v) {
                Ok(v) => Ok(Decimal(v)),
                Err(_) => Err(Error::out_of_range()),
            },
        }
    }
}

impl TryFrom<isize> for Decimal {
    type Error = Error;

    fn try_from(v: isize) -> Result<Decimal, Error> {
        match v.checked_mul(1000) {
            None => Err(Error::out_of_range()),
            Some(v) => match Integer::try_from(v) {
                Ok(v) => Ok(Decimal(v)),
                Err(_) => Err(Error::out_of_range()),
            },
        }
    }
}

impl From<u8> for Decimal {
    fn from(v: u8) -> Decimal {
        Self(Integer::from(v as u16 * 1000))
    }
}

impl From<u16> for Decimal {
    fn from(v: u16) -> Decimal {
        Self(Integer::from(v as u32 * 1000))
    }
}

impl From<u32> for Decimal {
    fn from(v: u32) -> Decimal {
        Self(Integer::try_from(v as u64 * 1000).unwrap())
    }
}

impl TryFrom<u64> for Decimal {
    type Error = Error;

    fn try_from(v: u64) -> Result<Decimal, Error> {
        match v.checked_mul(1000) {
            None => Err(Error::out_of_range()),
            Some(v) => match Integer::try_from(v) {
                Ok(v) => Ok(Decimal(v)),
                Err(_) => Err(Error::out_of_range()),
            },
        }
    }
}

impl TryFrom<u128> for Decimal {
    type Error = Error;

    fn try_from(v: u128) -> Result<Decimal, Error> {
        match v.checked_mul(1000) {
            None => Err(Error::out_of_range()),
            Some(v) => match Integer::try_from(v) {
                Ok(v) => Ok(Decimal(v)),
                Err(_) => Err(Error::out_of_range()),
            },
        }
    }
}

impl TryFrom<usize> for Decimal {
    type Error = Error;

    fn try_from(v: usize) -> Result<Decimal, Error> {
        match v.checked_mul(1000) {
            None => Err(Error::out_of_range()),
            Some(v) => match Integer::try_from(v) {
                Ok(v) => Ok(Decimal(v)),
                Err(_) => Err(Error::out_of_range()),
            },
        }
    }
}

impl From<Decimal> for f64 {
    fn from(v: Decimal) -> f64 {
        let v = i64::from(v.as_integer_scaled_1000());
        (v as f64) / 1000.0
    }
}

impl TryFrom<f32> for Decimal {
    type Error = Error;

    fn try_from(v: f32) -> Result<Decimal, Error> {
        (v as f64).try_into()
    }
}

impl TryFrom<f64> for Decimal {
    type Error = Error;

    fn try_from(v: f64) -> Result<Decimal, Error> {
        if v.is_nan() {
            return Err(Error::new("NaN"));
        }

        match Integer::try_from((v * 1000.0).round_ties_even() as i64) {
            Ok(v) => Ok(Decimal(v)),
            Err(_) => Err(Error::out_of_range()),
        }
    }
}

impl TryFrom<Integer> for Decimal {
    type Error = Error;

    fn try_from(v: Integer) -> Result<Decimal, Error> {
        i64::from(v).try_into()
    }
}
