use crate::{serializer::Serializer, Num, Parser};
use std::{convert::TryFrom, fmt, ops::Deref};

#[derive(Debug, PartialEq, Clone)]
pub struct Decimal(pub(crate) rust_decimal::Decimal);

impl TryFrom<rust_decimal::Decimal> for Decimal {
    type Error = &'static str;
    fn try_from(value: rust_decimal::Decimal) -> Result<Self, Self::Error> {
        let mut output = String::new();
        Serializer::serialize_decimal(value, &mut output)?;

        Ok(Decimal(value))
    }
}

impl Deref for Decimal {
    type Target = rust_decimal::Decimal;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Integers have a range of -999,999,999,999,999 to 999,999,999,999,999 inclusive (i.e., up to fifteen digits, signed), for IEEE 754 compatibility.
///
/// The ABNF for Integers is:
/// ```abnf,ignore,no_run
/// sf-integer = ["-"] 1*15DIGIT
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct Integer(pub(crate) i64);

impl Deref for Integer {
    type Target = i64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<i64> for Integer {
    type Error = &'static str;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        let input_string = value.to_string();
        let mut input_chars = input_string.chars().peekable();
        let validated = Parser::parse_number(&mut input_chars)?;
        match validated {
            Num::Integer(val) => Ok(Integer(val)),
            Num::Decimal(_) => Err("Input is Decimal, expected Integer"),
        }
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// TODO: how to get around naming collision without using std::string::String everywhere?
/// Strings are zero or more printable ASCII (RFC0020) characters (i.e., the range %x20 to %x7E). Note that this excludes tabs, newlines, carriage returns, etc.
///
/// The ABNF for Strings is:
/// ```abnf,ignore,no_run
/// sf-string = DQUOTE *chr DQUOTE
/// chr       = unescaped / escaped
/// unescaped = %x20-21 / %x23-5B / %x5D-7E
/// escaped   = "\" ( DQUOTE / "\" )
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct BareItemString(pub(crate) std::string::String);

impl Deref for BareItemString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<String> for BareItemString {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut output = String::new();
        Serializer::serialize_string(&value, &mut output)?;

        Ok(BareItemString(value))
    }
}

impl fmt::Display for BareItemString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Byte Sequences can be conveyed in Structured Fields.
///
/// The ABNF for a Byte Sequence is:
/// ```abnf,ignore,no_run
/// sf-binary = ":" *(base64) ":"
/// base64    = ALPHA / DIGIT / "+" / "/" / "="
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct ByteSeq(pub(crate) Vec<u8>);

impl From<&[u8]> for ByteSeq {
    fn from(value: &[u8]) -> Self {
        ByteSeq(value.to_vec())
    }
}

impl From<Vec<u8>> for ByteSeq {
    fn from(value: Vec<u8>) -> Self {
        ByteSeq(value)
    }
}

impl Deref for ByteSeq {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

/// Boolean values can be conveyed in Structured Fields.
///
/// The ABNF for a Boolean is:
/// ```abnf,ignore,no_run
/// sf-boolean = "?" boolean
/// boolean    = "0" / "1"
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct Boolean(pub(crate) bool);

impl From<bool> for Boolean {
    fn from(value: bool) -> Self {
        Boolean(value)
    }
}

impl Deref for Boolean {
    type Target = bool;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for Boolean {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Tokens are short textual words; their abstract model is identical to their expression in the HTTP field value serialization.
///
/// The ABNF for Tokens is:
/// ```abnf,ignore,no_run
/// sf-token = ( ALPHA / "*" ) *( tchar / ":" / "/" )
/// ```
///
/// # Example
/// ```
/// use sfv::{BareItem, Token};
/// use std::convert::{TryFrom, TryInto};
///
/// # fn main() -> Result<(), &'static str> {
/// let token_try_from = Token::try_from("foo")?;
/// let item = BareItem::Token(token_try_from);
///
/// let str_try_into: Token = "bar".try_into()?;
/// let item = BareItem::Token(str_try_into);
///
/// let direct_item_construction = BareItem::Token("baz".try_into()?);
/// # Ok(())
/// # }
/// ```
///
/// ```compile_fail
/// Token("foo"); // A Token can not be constructed directly
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct Token(pub(crate) String);

impl Deref for Token {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl TryFrom<String> for Token {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut output = String::new();
        Serializer::serialize_token(&value, &mut output)?;

        Ok(Token(value))
    }
}

impl TryFrom<&str> for Token {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut output = String::new();
        Serializer::serialize_token(&value, &mut output)?;
        Ok(Token(value.to_owned()))
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;
    use std::error::Error;
    use std::str::FromStr;

    use super::*;

    #[test]
    fn create_non_ascii_string_errors() -> Result<(), Box<dyn Error>> {
        let disallowed_value: Result<BareItemString, &str> =
            "non-ascii text 🐹".to_owned().try_into();

        assert_eq!(
            Err("serialize_string: non-ascii character"),
            disallowed_value
        );

        Ok(())
    }

    #[test]
    fn create_too_long_decimal_errors() -> Result<(), Box<dyn Error>> {
        let disallowed_value: Result<Decimal, &str> =
            rust_decimal::Decimal::from_str("12345678912345.123")?.try_into();
        assert_eq!(
            Err("serialize_decimal: integer component > 12 digits"),
            disallowed_value
        );

        Ok(())
    }
}
