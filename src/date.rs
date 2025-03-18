use crate::Integer;

use std::fmt;

/// A structured field value [date].
///
/// Dates represent an integer number of seconds from the Unix epoch.
///
/// [`Format::Rfc9651`][`crate::Format::Rfc9651`] supports bare items of this
/// type; [`Format::Rfc8941`][`crate::Format::Rfc8941`] does not.
///
/// [date]: <https://httpwg.org/specs/rfc9651.html#date>
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Date(Integer);

impl Date {
    /// The minimum value for a parsed or serialized date.
    pub const MIN: Self = Self(Integer::MIN);

    /// The maximum value for a parsed or serialized date.
    pub const MAX: Self = Self(Integer::MAX);

    /// The Unix epoch: `1970-01-01T00:00:00Z`.
    pub const UNIX_EPOCH: Self = Self(Integer::ZERO);

    /// Returns the date as an integer number of seconds from the Unix epoch.
    pub fn unix_seconds(&self) -> Integer {
        self.0
    }

    /// Creates a date from an integer number of seconds from the Unix epoch.
    pub const fn from_unix_seconds(v: Integer) -> Self {
        Self(v)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}
