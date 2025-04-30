use std::borrow::Borrow;
use std::fmt;

use crate::error::{Error, NonEmptyStringError};
use crate::utils;

/// An owned structured field value [token].
///
/// Tokens must match the following regular expression:
///
/// ```re
/// ^[A-Za-z*][A-Za-z*0-9!#$%&'+\-.^_`|~]*$
/// ```
///
/// [token]: <https://httpwg.org/specs/rfc9651.html#token>
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token(String);

/// A borrowed structured field value [token].
///
/// Tokens must match the following regular expression:
///
/// ```re
/// ^[A-Za-z*][A-Za-z*0-9!#$%&'+\-.^_`|~]*$
/// ```
///
/// This type is to [`Token`] as [`str`] is to [`String`].
///
/// [token]: <https://httpwg.org/specs/rfc9651.html#token>
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, ref_cast::RefCastCustom)]
#[repr(transparent)]
pub struct TokenRef(str);

const fn validate(v: &[u8]) -> Result<(), NonEmptyStringError> {
    if v.is_empty() {
        return Err(NonEmptyStringError::empty());
    }

    if !utils::is_allowed_start_token_char(v[0]) {
        return Err(NonEmptyStringError::invalid_character(0));
    }

    let mut index = 1;

    while index < v.len() {
        if !utils::is_allowed_inner_token_char(v[index]) {
            return Err(NonEmptyStringError::invalid_character(index));
        }
        index += 1;
    }

    Ok(())
}

impl TokenRef {
    #[ref_cast::ref_cast_custom]
    const fn cast(v: &str) -> &Self;

    /// Creates a `&TokenRef` from a `&str`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(v: &str) -> Result<&Self, Error> {
        validate(v.as_bytes())?;
        Ok(Self::cast(v))
    }

    // Like `from_str`, but assumes that the contents of the string have already
    // been validated as a token.
    pub(crate) fn from_validated_str(v: &str) -> &Self {
        debug_assert!(validate(v.as_bytes()).is_ok());
        Self::cast(v)
    }

    /// Creates a `&TokenRef`, panicking if the value is invalid.
    ///
    /// This method is intended to be called from `const` contexts in which the
    /// value is known to be valid. Use [`TokenRef::from_str`] for non-panicking
    /// conversions.
    pub const fn constant(v: &str) -> &Self {
        match validate(v.as_bytes()) {
            Ok(_) => Self::cast(v),
            Err(err) => panic!("{}", err.msg()),
        }
    }

    /// Returns the token as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ToOwned for TokenRef {
    type Owned = Token;

    fn to_owned(&self) -> Token {
        Token(self.0.to_owned())
    }

    fn clone_into(&self, target: &mut Token) {
        self.0.clone_into(&mut target.0);
    }
}

impl Borrow<TokenRef> for Token {
    fn borrow(&self) -> &TokenRef {
        self
    }
}

impl std::ops::Deref for Token {
    type Target = TokenRef;

    fn deref(&self) -> &TokenRef {
        TokenRef::cast(&self.0)
    }
}

impl From<Token> for String {
    fn from(v: Token) -> String {
        v.0
    }
}

impl TryFrom<String> for Token {
    type Error = Error;

    fn try_from(v: String) -> Result<Token, Error> {
        validate(v.as_bytes())?;
        Ok(Token(v))
    }
}

impl Token {
    /// Creates a `Token` from a `String`.
    ///
    /// Returns the original value if the conversion failed.
    pub fn from_string(v: String) -> Result<Self, (Error, String)> {
        match validate(v.as_bytes()) {
            Ok(_) => Ok(Self(v)),
            Err(err) => Err((err.into(), v)),
        }
    }
}

/// Creates a `&TokenRef`, panicking if the value is invalid.
///
/// This is a convenience free function for [`TokenRef::constant`].
///
/// This method is intended to be called from `const` contexts in which the
/// value is known to be valid. Use [`TokenRef::from_str`] for non-panicking
/// conversions.
pub const fn token_ref(v: &str) -> &TokenRef {
    TokenRef::constant(v)
}

impl fmt::Display for TokenRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <TokenRef as fmt::Display>::fmt(self, f)
    }
}

macro_rules! impl_eq {
    ($a: ty, $b: ty) => {
        impl PartialEq<$a> for $b {
            fn eq(&self, other: &$a) -> bool {
                <TokenRef as PartialEq>::eq(self, other)
            }
        }
        impl PartialEq<$b> for $a {
            fn eq(&self, other: &$b) -> bool {
                <TokenRef as PartialEq>::eq(self, other)
            }
        }
    };
}

impl_eq!(Token, TokenRef);
impl_eq!(Token, &TokenRef);

impl<'a> TryFrom<&'a str> for &'a TokenRef {
    type Error = Error;

    fn try_from(v: &'a str) -> Result<&'a TokenRef, Error> {
        TokenRef::from_str(v)
    }
}

impl Borrow<str> for Token {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for TokenRef {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for &'a TokenRef {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        TokenRef::from_str(<&str>::arbitrary(u)?).map_err(|_| arbitrary::Error::IncorrectFormat)
    }
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for Token {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        <&TokenRef>::arbitrary(u).map(ToOwned::to_owned)
    }
}
