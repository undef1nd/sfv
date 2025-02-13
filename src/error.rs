use std::fmt;

/// An error that occurs during parsing or serialization.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Error {
    msg: &'static str,
}

impl Error {
    pub(crate) fn new(msg: &'static str) -> Self {
        Self { msg }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.msg)
    }
}

impl std::error::Error for Error {}
