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

/// An error produced during conversion to a key.
#[derive(Debug)]
pub struct KeyError {
    byte_index: Option<usize>,
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(byte_index) = self.byte_index {
            write!(f, "invalid character for key at byte index {}", byte_index)
        } else {
            f.write_str("key cannot be empty")
        }
    }
}

impl std::error::Error for KeyError {}

const fn validate(v: &[u8]) -> Result<(), KeyError> {
    if v.is_empty() {
        return Err(KeyError { byte_index: None });
    }

    if !utils::is_allowed_start_key_char(v[0]) {
        return Err(KeyError {
            byte_index: Some(0),
        });
    }

    let mut index = 1;

    while index < v.len() {
        if !utils::is_allowed_inner_key_char(v[index]) {
            return Err(KeyError {
                byte_index: Some(index),
            });
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
    pub fn from_str(v: &str) -> Result<&Self, KeyError> {
        validate(v.as_bytes())?;
        Ok(Self::cast(v))
    }

    /// Creates a `&KeyRef`, panicking if the value is invalid.
    ///
    /// This method is intended to be called from `const` contexts in which the
    /// value is known to be valid. Use [`KeyRef::from_str`] for non-panicking
    /// conversions.
    pub const fn constant(v: &str) -> &Self {
        match validate(v.as_bytes()) {
            Ok(_) => Self::cast(v),
            Err(err) => {
                if err.byte_index.is_none() {
                    panic!("key cannot be empty")
                } else {
                    panic!("invalid character for key")
                }
            }
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
    type Error = KeyError;

    fn try_from(v: String) -> Result<Key, KeyError> {
        validate(v.as_bytes())?;
        Ok(Key(v))
    }
}

impl Key {
    /// Creates a `Key` from a `String`.
    ///
    /// Returns the original value if the conversion failed.
    pub fn from_string(v: String) -> Result<Self, (KeyError, String)> {
        match validate(v.as_bytes()) {
            Ok(_) => Ok(Self(v)),
            Err(err) => Err((err, v)),
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
    type Error = KeyError;

    fn try_from(v: &'a str) -> Result<&'a KeyRef, KeyError> {
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
