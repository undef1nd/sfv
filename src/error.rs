use std::fmt;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) enum Repr {
    Visit(Box<str>),

    OutOfRange,
    NaN,

    Empty,
    InvalidCharacter(usize),

    TrailingCharactersAfterMember(usize),
    TrailingComma(usize),
    TrailingCharactersAfterParsedValue(usize),

    ExpectedStartOfInnerList(usize),
    ExpectedInnerListDelimiter(usize),
    UnterminatedInnerList(usize),

    ExpectedStartOfBareItem(usize),

    ExpectedStartOfBoolean(usize),
    ExpectedBoolean(usize),

    ExpectedStartOfString(usize),
    InvalidStringCharacter(usize),
    UnterminatedString(usize),

    UnterminatedEscapeSequence(usize),
    InvalidEscapeSequence(usize),

    ExpectedStartOfToken(usize),

    ExpectedStartOfByteSequence(usize),
    UnterminatedByteSequence(usize),
    InvalidByteSequence(usize),

    ExpectedDigit(usize),
    TooManyDigits(usize),
    TooManyDigitsBeforeDecimalPoint(usize),
    TooManyDigitsAfterDecimalPoint(usize),
    TrailingDecimalPoint(usize),

    ExpectedStartOfDate(usize),
    Rfc8941Date(usize),
    NonIntegerDate(usize),

    ExpectedStartOfDisplayString(usize),
    Rfc8941DisplayString(usize),
    ExpectedQuote(usize),
    InvalidUtf8InDisplayString(usize),
    InvalidDisplayStringCharacter(usize),
    UnterminatedDisplayString(usize),

    ExpectedStartOfKey(usize),
}

impl<E: std::error::Error> From<E> for Repr {
    fn from(err: E) -> Self {
        Self::Visit(err.to_string().into())
    }
}

impl fmt::Display for Repr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (msg, index) = match *self {
            Self::Visit(ref msg) => return f.write_str(msg),

            Self::NaN => return f.write_str("NaN"),
            Self::OutOfRange => return f.write_str("out of range"),

            Self::Empty => return f.write_str("cannot be empty"),
            Self::InvalidCharacter(i) => ("invalid character", i),

            Self::TrailingCharactersAfterMember(i) => ("trailing characters after member", i),
            Self::TrailingComma(i) => ("trailing comma", i),
            Self::TrailingCharactersAfterParsedValue(i) => {
                ("trailing characters after parsed value", i)
            }

            Self::ExpectedStartOfInnerList(i) => ("expected start of inner list", i),
            Self::ExpectedInnerListDelimiter(i) => {
                ("expected inner list delimiter (' ' or ')')", i)
            }
            Self::UnterminatedInnerList(i) => ("unterminated inner list", i),

            Self::ExpectedStartOfBareItem(i) => ("expected start of bare item", i),

            Self::ExpectedStartOfBoolean(i) => ("expected start of boolean ('?')", i),
            Self::ExpectedBoolean(i) => ("expected boolean ('0' or '1')", i),

            Self::ExpectedStartOfString(i) => (r#"expected start of string ('"')"#, i),
            Self::InvalidStringCharacter(i) => ("invalid string character", i),
            Self::UnterminatedString(i) => ("unterminated string", i),

            Self::UnterminatedEscapeSequence(i) => ("unterminated escape sequence", i),
            Self::InvalidEscapeSequence(i) => ("invalid escape sequence", i),

            Self::ExpectedStartOfToken(i) => ("expected start of token", i),

            Self::ExpectedStartOfByteSequence(i) => ("expected start of byte sequence (':')", i),
            Self::UnterminatedByteSequence(i) => ("unterminated byte sequence", i),
            Self::InvalidByteSequence(i) => ("invalid byte sequence", i),

            Self::ExpectedDigit(i) => ("expected digit", i),
            Self::TooManyDigits(i) => ("too many digits", i),
            Self::TooManyDigitsBeforeDecimalPoint(i) => ("too many digits before decimal point", i),
            Self::TooManyDigitsAfterDecimalPoint(i) => ("too many digits after decimal point", i),
            Self::TrailingDecimalPoint(i) => ("trailing decimal point", i),

            Self::ExpectedStartOfDate(i) => ("expected start of date ('@')", i),
            Self::Rfc8941Date(i) => ("RFC 8941 does not support dates", i),
            Self::NonIntegerDate(i) => ("date must be an integer number of seconds", i),

            Self::ExpectedStartOfDisplayString(i) => ("expected start of display string ('%')", i),
            Self::Rfc8941DisplayString(i) => ("RFC 8941 does not support display strings", i),
            Self::ExpectedQuote(i) => (r#"expected '"'"#, i),
            Self::InvalidUtf8InDisplayString(i) => ("invalid UTF-8 in display string", i),
            Self::InvalidDisplayStringCharacter(i) => ("invalid display string character", i),
            Self::UnterminatedDisplayString(i) => ("unterminated display string", i),

            Self::ExpectedStartOfKey(i) => ("expected start of key ('a'-'z' or '*')", i),
        };

        write!(f, "{msg} at index {index}")
    }
}

/// An error that can occur in this crate.
///
/// The most common type of error is invalid input during parsing, but others
/// exist as well:
///
/// - Conversion to or from bare-item types such as [`Integer`][crate::Integer]
/// - Attempting to serialize an empty [list][crate::ListSerializer::finish] or
///   [dictionary][crate::DictSerializer::finish]
///
/// Other than implementing the [`std::error::Error`], [`std::fmt::Debug`], and
/// [`std::fmt::Display`] traits, this error type currently provides no
/// introspection capabilities.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Error {
    repr: Repr,
}

impl From<Repr> for Error {
    fn from(repr: Repr) -> Self {
        Self { repr }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.repr, f)
    }
}

impl std::error::Error for Error {}

pub(crate) struct NonEmptyStringError {
    byte_index: Option<usize>,
}

impl NonEmptyStringError {
    pub(crate) const fn empty() -> Self {
        Self { byte_index: None }
    }

    pub(crate) const fn invalid_character(byte_index: usize) -> Self {
        Self {
            byte_index: Some(byte_index),
        }
    }

    pub const fn msg(&self) -> &'static str {
        match self.byte_index {
            None => "cannot be empty",
            Some(_) => "invalid character",
        }
    }
}

impl From<NonEmptyStringError> for Error {
    fn from(err: NonEmptyStringError) -> Self {
        match err.byte_index {
            None => Repr::Empty,
            Some(index) => Repr::InvalidCharacter(index),
        }
        .into()
    }
}
