use std::fmt;

/// An error that occurs during parsing or serialization.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Error {
    msg: &'static str,
    index: Option<usize>,
}

impl Error {
    pub(crate) fn new(msg: &'static str) -> Self {
        Self { msg, index: None }
    }

    pub(crate) fn with_index(msg: &'static str, index: usize) -> Self {
        Self {
            msg,
            index: Some(index),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.index {
            None => f.write_str(self.msg),
            Some(index) => write!(f, "{} at index {}", self.msg, index),
        }
    }
}

impl std::error::Error for Error {}
