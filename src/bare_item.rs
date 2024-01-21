use crate::serializer::Serializer;
use std::{
    convert::{TryFrom, TryInto},
    fmt,
    ops::Deref,
};

#[cfg(feature = "sf-date-item")]
use chrono::NaiveDateTime;

/// `BareItem` type is used to construct `Items` or `Parameters` values.
#[non_exhaustive]
#[derive(Debug, PartialEq, Clone)]
pub enum BareItem {
    /// Decimal number
    // sf-decimal  = ["-"] 1*12DIGIT "." 1*3DIGIT
    Decimal(Decimal),
    /// Integer number
    // sf-integer = ["-"] 1*15DIGIT
    Integer(Integer),
    // sf-string = DQUOTE *chr DQUOTE
    // chr       = unescaped / escaped
    // unescaped = %x20-21 / %x23-5B / %x5D-7E
    // escaped   = "\" ( DQUOTE / "\" )
    String(BareItemString),
    // ":" *(base64) ":"
    // base64    = ALPHA / DIGIT / "+" / "/" / "="
    ByteSeq(ByteSeq),
    // sf-boolean = "?" boolean
    // boolean    = "0" / "1"
    Boolean(Boolean),
    // sf-token = ( ALPHA / "*" ) *( tchar / ":" / "/" )
    Token(Token),
    #[cfg(feature = "sf-date-item")]
    Date(Date),
}

impl BareItem {
    /// If `BareItem` is a decimal, returns `Decimal`, otherwise returns `None`.
    /// ```
    /// # use sfv::{BareItem, FromPrimitive};
    /// use rust_decimal::Decimal;
    /// # use std::convert::TryInto;
    /// # fn main() -> Result<(), &'static str> {
    /// let decimal_number = Decimal::from_f64(415.566).unwrap();
    /// let bare_item: BareItem = decimal_number.try_into()?;
    /// assert_eq!(bare_item.as_decimal().unwrap(), decimal_number);
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_decimal(&self) -> Option<rust_decimal::Decimal> {
        match self {
            BareItem::Decimal(val) => Some(val.0),
            _ => None,
        }
    }
    /// If `BareItem` is an integer, returns `i64`, otherwise returns `None`.
    /// ```
    /// # use sfv::BareItem;
    /// # use std::convert::TryInto;
    /// # fn main() -> Result<(), &'static str> {
    /// let bare_item: BareItem = 100_i64.try_into()?;
    /// assert_eq!(bare_item.as_int().unwrap(), 100);
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_int(&self) -> Option<i64> {
        match &self {
            BareItem::Integer(val) => Some(**val),
            _ => None,
        }
    }
    /// If `BareItem` is `String`, returns `&str`, otherwise returns `None`.
    /// ```
    /// # use sfv::BareItem;
    /// # use std::convert::TryInto;
    /// # fn main() -> Result<(), &'static str> {
    /// let bare_item = BareItem::String("foo".to_owned().try_into()?);
    /// assert_eq!(bare_item.as_str().unwrap(), "foo");
    /// Ok(())
    /// # }
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            BareItem::String(ref val) => Some(val),
            _ => None,
        }
    }
    /// If `BareItem` is a `ByteSeq`, returns `&Vec<u8>`, otherwise returns `None`.
    /// ```
    /// # use sfv::BareItem;
    /// let bare_item = BareItem::ByteSeq("foo".to_owned().into_bytes().into());
    /// assert_eq!(bare_item.as_byte_seq().unwrap().as_slice(), "foo".as_bytes());
    /// ```
    pub fn as_byte_seq(&self) -> Option<&Vec<u8>> {
        match *self {
            BareItem::ByteSeq(ref val) => Some(&val.0),
            _ => None,
        }
    }
    /// If `BareItem` is a `Boolean`, returns `bool`, otherwise returns `None`.
    /// ```
    /// # use sfv::{BareItem, Decimal, FromPrimitive};
    /// let bare_item = BareItem::Boolean(true.into());
    /// assert_eq!(bare_item.as_bool().unwrap(), true);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            BareItem::Boolean(val) => Some(val.0),
            _ => None,
        }
    }
    /// If `BareItem` is a `Token`, returns `&str`, otherwise returns `None`.
    /// ```
    /// use sfv::BareItem;
    /// # use std::convert::TryInto;
    /// # fn main() -> Result<(), &'static str> {
    ///
    /// let bare_item = BareItem::Token("*bar".try_into()?);
    /// assert_eq!(bare_item.as_token().unwrap(), "*bar");
    /// # Ok(())
    /// # }
    /// ```
    pub fn as_token(&self) -> Option<&str> {
        match *self {
            BareItem::Token(ref val) => Some(val),
            _ => None,
        }
    }
}

impl TryFrom<i64> for BareItem {
    type Error = &'static str;
    /// Converts `i64` into `BareItem::Integer`.
    /// ```
    /// # use sfv::BareItem;
    /// # use std::convert::TryInto;
    /// # fn main() -> Result<(), &'static str> {
    /// let bare_item: BareItem = 456_i64.try_into()?;
    /// assert_eq!(bare_item.as_int().unwrap(), 456);
    /// # Ok(())
    /// # }
    /// ```
    fn try_from(item: i64) -> Result<Self, Self::Error> {
        Ok(BareItem::Integer(item.try_into()?))
    }
}

impl TryFrom<rust_decimal::Decimal> for BareItem {
    type Error = &'static str;
    /// Converts `rust_decimal::Decimal` into `BareItem::Decimal`.
    /// ```
    /// # use sfv::{BareItem, FromPrimitive};
    /// # use std::convert::TryInto;
    /// use rust_decimal::Decimal;
    /// # fn main() -> Result<(), &'static str> {
    /// let decimal_number = Decimal::from_f64(48.01).unwrap();
    /// let bare_item: BareItem = decimal_number.try_into()?;
    /// assert_eq!(bare_item.as_decimal().unwrap(), decimal_number);
    /// # Ok(())
    /// # }
    /// ```
    fn try_from(item: rust_decimal::Decimal) -> Result<Self, Self::Error> {
        Ok(BareItem::Decimal(item.try_into()?))
    }
}

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
        let mut output = String::new();
        Serializer::serialize_integer(value, &mut output)?;
        Ok(Integer(value))
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "sf-date-item")]
/// Dates have a data model that is similar to Integers, representing a (possibly negative) delta in seconds from January 1, 1970 00:00:00 UTC, excluding leap seconds.
///
/// The ABNF for Dates is:
/// ```abnf,ignore,no_run
/// sf-date = "@" ["-"] 1*15DIGIT
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct Date(pub(crate) NaiveDateTime);

#[cfg(feature = "sf-date-item")]
impl TryFrom<NaiveDateTime> for Date {
    type Error = &'static str;
    fn try_from(value: NaiveDateTime) -> Result<Self, Self::Error> {
        let mut output = String::new();
        Serializer::serialize_date(value, &mut output)?;

        Ok(Date(value))
    }
}

#[cfg(feature = "sf-date-item")]
impl Deref for Date {
    type Target = NaiveDateTime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "sf-date-item")]
impl fmt::Display for Date {
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
