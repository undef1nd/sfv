use crate::error::{Error, NonEmptyStringError};
use crate::utils;

use std::borrow::Borrow;
use std::convert::TryFrom;
use std::fmt;

/// An owned structured field value [key].
///
/// Keys must match the following regular expression:
///
/// ```re
/// ^[A-Za-z*][A-Za-z*0-9!#$%&'+\-.^_`|~]*$
/// ```
///
/// [key]: <https://httpwg.org/specs/rfc8941.html#key>
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(String);

/// A borrowed structured field value [key].
///
/// Keys must match the following regular expression:
///
/// ```re
/// ^[A-Za-z*][A-Za-z*0-9!#$%&'+\-.^_`|~]*$
/// ```
///
/// This type is to [`Key`] as [`str`] is to [`String`].
///
/// [key]: <https://httpwg.org/specs/rfc8941.html#key>
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, ref_cast::RefCastCustom)]
#[repr(transparent)]
pub struct KeyRef(str);

const fn validate(v: &[u8]) -> Result<(), NonEmptyStringError> {
    if v.is_empty() {
        return Err(NonEmptyStringError::empty());
    }

    if !utils::is_allowed_start_key_char(v[0]) {
        return Err(NonEmptyStringError::invalid_character(0));
    }

    let mut index = 1;

    while index < v.len() {
        if !utils::is_allowed_inner_key_char(v[index]) {
            return Err(NonEmptyStringError::invalid_character(index));
        }
        index += 1;
    }

    Ok(())
}

impl KeyRef {
    #[ref_cast::ref_cast_custom]
    const fn cast(v: &str) -> &Self;

    /// Creates a `&KeyRef` from a `&str`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(v: &str) -> Result<&Self, Error> {
        validate(v.as_bytes())?;
        Ok(Self::cast(v))
    }

    // Like `from_str`, but assumes that the contents of the string have already
    // been validated as a key.
    pub(crate) fn from_validated_str(v: &str) -> &Self {
        debug_assert!(validate(v.as_bytes()).is_ok());
        Self::cast(v)
    }

    /// Creates a `&KeyRef`, panicking if the value is invalid.
    ///
    /// This method is intended to be called from `const` contexts in which the
    /// value is known to be valid. Use [`KeyRef::from_str`] for non-panicking
    /// conversions.
    pub const fn constant(v: &str) -> &Self {
        match validate(v.as_bytes()) {
            Ok(_) => Self::cast(v),
            Err(err) => panic!("{}", err.msg()),
        }
    }

    /// Returns the key as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ToOwned for KeyRef {
    type Owned = Key;

    fn to_owned(&self) -> Key {
        Key(self.0.to_owned())
    }

    fn clone_into(&self, target: &mut Key) {
        self.0.clone_into(&mut target.0);
    }
}

impl Borrow<KeyRef> for Key {
    fn borrow(&self) -> &KeyRef {
        self
    }
}

impl std::ops::Deref for Key {
    type Target = KeyRef;

    fn deref(&self) -> &KeyRef {
        KeyRef::cast(&self.0)
    }
}

impl From<Key> for String {
    fn from(v: Key) -> String {
        v.0
    }
}

impl TryFrom<String> for Key {
    type Error = Error;

    fn try_from(v: String) -> Result<Key, Error> {
        validate(v.as_bytes())?;
        Ok(Key(v))
    }
}

impl Key {
    /// Creates a `Key` from a `String`.
    ///
    /// Returns the original value if the conversion failed.
    pub fn from_string(v: String) -> Result<Self, (Error, String)> {
        match validate(v.as_bytes()) {
            Ok(_) => Ok(Self(v)),
            Err(err) => Err((err.into(), v)),
        }
    }
}

/// Creates a `&KeyRef`, panicking if the value is invalid.
///
/// This is a convenience free function for [`KeyRef::constant`].
///
/// This method is intended to be called from `const` contexts in which the
/// value is known to be valid. Use [`KeyRef::from_str`] for non-panicking
/// conversions.
pub const fn key_ref(v: &str) -> &KeyRef {
    KeyRef::constant(v)
}

impl fmt::Display for KeyRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <KeyRef as fmt::Display>::fmt(self, f)
    }
}

macro_rules! impl_eq {
    ($a: ty, $b: ty) => {
        impl PartialEq<$a> for $b {
            fn eq(&self, other: &$a) -> bool {
                <KeyRef as PartialEq>::eq(self, other)
            }
        }
        impl PartialEq<$b> for $a {
            fn eq(&self, other: &$b) -> bool {
                <KeyRef as PartialEq>::eq(self, other)
            }
        }
    };
}

impl_eq!(Key, KeyRef);
impl_eq!(Key, &KeyRef);

impl<'a> TryFrom<&'a str> for &'a KeyRef {
    type Error = Error;

    fn try_from(v: &'a str) -> Result<&'a KeyRef, Error> {
        KeyRef::from_str(v)
    }
}

impl Borrow<str> for Key {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for KeyRef {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<KeyRef> for Key {
    fn as_ref(&self) -> &KeyRef {
        self
    }
}

impl AsRef<KeyRef> for KeyRef {
    fn as_ref(&self) -> &KeyRef {
        self
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for &'a KeyRef {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        KeyRef::from_str(<&str>::arbitrary(u)?).map_err(|_| arbitrary::Error::IncorrectFormat)
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (1, None)
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for Key {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        <&KeyRef>::arbitrary(u).map(ToOwned::to_owned)
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (1, None)
    }
}
