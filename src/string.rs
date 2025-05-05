use std::{
    borrow::{Borrow, Cow},
    fmt,
    string::String as StdString,
};

use crate::Error;

/// An owned structured field value [string].
///
/// Strings may only contain printable ASCII characters (i.e. the range
/// `0x20 ..= 0x7e`).
///
/// [string]: <https://httpwg.org/specs/rfc9651.html#string>
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct String(StdString);

/// A borrowed structured field value [string].
///
/// Strings may only contain printable ASCII characters (i.e. the range
/// `0x20 ..= 0x7e`).
///
/// This type is to [`String`] as [`str`] is to [`std::string::String`].
///
/// [string]: <https://httpwg.org/specs/rfc9651.html#string>
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, ref_cast::RefCastCustom)]
#[repr(transparent)]
pub struct StringRef(str);

struct StringError {
    byte_index: usize,
}

impl From<StringError> for Error {
    fn from(err: StringError) -> Error {
        Error::with_index("invalid character", err.byte_index)
    }
}

const fn validate(v: &[u8]) -> Result<(), StringError> {
    let mut index = 0;

    while index < v.len() {
        if v[index] < 0x20 || v[index] > 0x7e {
            return Err(StringError { byte_index: index });
        }
        index += 1;
    }

    Ok(())
}

impl StringRef {
    #[ref_cast::ref_cast_custom]
    const fn cast(v: &str) -> &Self;

    /// Creates an empty `&StringRef`.
    #[must_use]
    pub const fn empty() -> &'static Self {
        Self::cast("")
    }

    /// Creates a `&StringRef` from a `&str`.
    ///
    /// # Errors
    /// The error reports the cause of any failed string validation.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(v: &str) -> Result<&Self, Error> {
        validate(v.as_bytes())?;
        Ok(Self::cast(v))
    }

    /// Creates a `&StringRef`, panicking if the value is invalid.
    ///
    /// This method is intended to be called from `const` contexts in which the
    /// value is known to be valid. Use [`StringRef::from_str`] for non-panicking
    /// conversions.
    #[must_use]
    pub const fn constant(v: &str) -> &Self {
        match validate(v.as_bytes()) {
            Ok(()) => Self::cast(v),
            Err(_) => panic!("invalid character for string"),
        }
    }

    /// Returns the string as a `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ToOwned for StringRef {
    type Owned = String;

    fn to_owned(&self) -> String {
        String(self.0.to_owned())
    }

    fn clone_into(&self, target: &mut String) {
        self.0.clone_into(&mut target.0);
    }
}

impl Borrow<StringRef> for String {
    fn borrow(&self) -> &StringRef {
        self
    }
}

impl std::ops::Deref for String {
    type Target = StringRef;

    fn deref(&self) -> &StringRef {
        StringRef::cast(&self.0)
    }
}

impl From<String> for StdString {
    fn from(v: String) -> StdString {
        v.0
    }
}

impl TryFrom<StdString> for String {
    type Error = Error;

    fn try_from(v: StdString) -> Result<String, Error> {
        validate(v.as_bytes())?;
        Ok(String(v))
    }
}

impl String {
    /// Creates a `String` from a `std::string::String`.
    ///
    /// Returns the original value if the conversion failed.
    ///
    /// # Errors
    /// The error reports any problems from failed validation.
    pub fn from_string(v: StdString) -> Result<Self, (Error, StdString)> {
        match validate(v.as_bytes()) {
            Ok(()) => Ok(Self(v)),
            Err(err) => Err((err.into(), v)),
        }
    }
}

/// Creates a `&StringRef`, panicking if the value is invalid.
///
/// This is a convenience free function for [`StringRef::constant`].
///
/// This method is intended to be called from `const` contexts in which the
/// value is known to be valid. Use [`StringRef::from_str`] for non-panicking
/// conversions.
#[must_use]
pub const fn string_ref(v: &str) -> &StringRef {
    StringRef::constant(v)
}

impl fmt::Display for StringRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl fmt::Display for String {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <StringRef as fmt::Display>::fmt(self, f)
    }
}

macro_rules! impl_eq {
    ($a: ty, $b: ty) => {
        impl PartialEq<$a> for $b {
            fn eq(&self, other: &$a) -> bool {
                <StringRef as PartialEq>::eq(self, other)
            }
        }
        impl PartialEq<$b> for $a {
            fn eq(&self, other: &$b) -> bool {
                <StringRef as PartialEq>::eq(self, other)
            }
        }
    };
}

impl_eq!(String, StringRef);
impl_eq!(String, &StringRef);
impl_eq!(Cow<'_, StringRef>, StringRef);
impl_eq!(Cow<'_, StringRef>, &StringRef);

impl<'a> TryFrom<&'a str> for &'a StringRef {
    type Error = Error;

    fn try_from(v: &'a str) -> Result<&'a StringRef, Error> {
        StringRef::from_str(v)
    }
}

impl Borrow<str> for String {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for StringRef {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for &'a StringRef {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        StringRef::from_str(<&str>::arbitrary(u)?).map_err(|_| arbitrary::Error::IncorrectFormat)
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for String {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        <&StringRef>::arbitrary(u).map(ToOwned::to_owned)
    }
}

impl Default for &StringRef {
    fn default() -> Self {
        StringRef::empty()
    }
}
