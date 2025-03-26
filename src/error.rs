use std::borrow::Cow;
use std::fmt;

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
    msg: Cow<'static, str>,
    index: Option<usize>,
}

impl Error {
    pub(crate) fn new(msg: &'static str) -> Self {
        Self {
            msg: Cow::Borrowed(msg),
            index: None,
        }
    }

    pub(crate) fn with_index(msg: &'static str, index: usize) -> Self {
        Self {
            msg: Cow::Borrowed(msg),
            index: Some(index),
        }
    }

    pub(crate) fn out_of_range() -> Self {
        Self::new("out of range")
    }

    pub(crate) fn custom(msg: impl fmt::Display) -> Self {
        Self {
            msg: Cow::Owned(msg.to_string()),
            index: None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.index {
            None => f.write_str(&self.msg),
            Some(index) => write!(f, "{} at index {}", self.msg, index),
        }
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
    fn from(err: NonEmptyStringError) -> Error {
        Error {
            msg: Cow::Borrowed(err.msg()),
            index: err.byte_index,
        }
    }
}
