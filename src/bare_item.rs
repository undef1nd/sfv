use crate::{utils, SFVResult};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use std::{
    convert::{TryFrom, TryInto},
    fmt,
    ops::Deref,
};

/// `BareItem` type is used to construct `Items` or `Parameters` values.
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
}

impl BareItem {
    /// Creates a `BareItem::Decimal` from an `f64` input.
    /// ```
    /// # use sfv::BareItem;
    /// # fn main() -> Result<(), &'static str> {
    /// let value = BareItem::new_decimal_from_f64(13.37)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_decimal_from_f64(value: f64) -> SFVResult<BareItem> {
        let decimal = rust_decimal::Decimal::from_f64(value)
            .ok_or("validate_decimal: value can not represent decimal")?;

        Self::new_decimal(decimal)
    }

    /// Creates a `BareItem::Decimal` from a `rust_decimal::Decimal` input.
    /// ```
    /// # use sfv::BareItem;
    /// # use crate::sfv::FromPrimitive;
    /// # fn main() -> Result<(), &'static str> {
    /// let decimal = rust_decimal::Decimal::from_f64(13.37).unwrap();
    /// let value = BareItem::new_decimal(decimal);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_decimal(value: rust_decimal::Decimal) -> SFVResult<BareItem> {
        let value: Decimal = value.try_into()?;
        Ok(BareItem::Decimal(value))
    }

    /// Creates a `BareItem::Decimal` from a `rust_decimal::Decimal` input.
    /// ```
    /// # use sfv::BareItem;
    /// # fn main() -> Result<(), &'static str> {
    /// let value = BareItem::new_integer(42)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_integer(value: i64) -> SFVResult<BareItem> {
        let value: Integer = value.try_into()?;
        Ok(BareItem::Integer(value))
    }

    /// Creates a `BareItem::String` from a `&str` input.
    /// ```
    /// # use sfv::BareItem;
    /// # fn main() -> Result<(), &'static str> {
    /// let value = BareItem::new_string("foo")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_string(value: &str) -> SFVResult<BareItem> {
        let value: BareItemString = value.try_into()?;
        Ok(BareItem::String(value))
    }

    /// Creates a `BareItem::ByteSeq` from a byte slice input.
    /// ```
    /// # use sfv::BareItem;
    /// # fn main() -> Result<(), &'static str> {
    /// let value = BareItem::new_byte_seq("hello".as_bytes())?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_byte_seq(value: &[u8]) -> SFVResult<BareItem> {
        let value: ByteSeq = value.into();
        Ok(BareItem::ByteSeq(value))
    }

    /// Creates a `BareItem::Boolean` from a `bool` input.
    /// ```
    /// # use sfv::BareItem;
    /// # fn main() -> Result<(), &'static str> {
    /// let value = BareItem::new_boolean(true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_boolean(value: bool) -> SFVResult<BareItem> {
        let value: Boolean = value.into();
        Ok(BareItem::Boolean(value))
    }

    /// Creates a `BareItem::Token` from a `&str` input.
    /// ```
    /// # use sfv::BareItem;
    /// # fn main() -> Result<(), &'static str> {
    /// let value = BareItem::new_boolean(true)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new_token(value: &str) -> SFVResult<BareItem> {
        let value: Token = value.try_into()?;
        Ok(BareItem::Token(value))
    }
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
        Self::new_integer(item)
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
        Self::new_decimal(item)
    }
}

impl TryFrom<f64> for BareItem {
    type Error = &'static str;

    /// Converts `f64` into `BareItem::Decimal`.
    /// ```
    /// # use sfv::{BareItem, FromPrimitive};
    /// # use std::convert::TryInto;
    /// # use rust_decimal::prelude::ToPrimitive;
    /// # fn main() -> Result<(), &'static str> {
    /// let decimal_number = 48.01;
    /// let bare_item: BareItem = decimal_number.try_into()?;
    /// assert_eq!(bare_item.as_decimal().unwrap().to_f64().unwrap(), decimal_number);
    /// # Ok(())
    /// # }
    /// ```
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new_decimal_from_f64(value)
    }
}

impl TryFrom<&[u8]> for BareItem {
    type Error = &'static str;

    /// Converts a byte slice into `BareItem::ByteSeq`.
    /// ```
    /// # use sfv::{BareItem, FromPrimitive};
    /// # use std::convert::TryInto;
    /// # fn main() -> Result<(), &'static str> {
    /// let byte_slice = "hello".as_bytes();
    /// let bare_item: BareItem = byte_slice.try_into()?;
    /// assert_eq!(bare_item.as_byte_seq().unwrap(), byte_slice);
    /// # Ok(())
    /// # }
    /// ```
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::new_byte_seq(value)
    }
}

impl TryFrom<bool> for BareItem {
    type Error = &'static str;

    /// Converts a `bool` into `BareItem::Boolean`.
    /// ```
    /// # use sfv::{BareItem, FromPrimitive};
    /// # use std::convert::TryInto;
    /// # fn main() -> Result<(), &'static str> {
    /// let boolean = true;
    /// let bare_item: BareItem = boolean.try_into()?;
    /// assert_eq!(bare_item.as_bool().unwrap(), boolean);
    /// # Ok(())
    /// # }
    /// ```
    fn try_from(value: bool) -> Result<Self, Self::Error> {
        Self::new_boolean(value)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Decimal(pub(crate) rust_decimal::Decimal);

impl TryFrom<rust_decimal::Decimal> for Decimal {
    type Error = &'static str;
    fn try_from(value: rust_decimal::Decimal) -> Result<Self, Self::Error> {
        let validated = Self::validate(value)?;
        Ok(Decimal(validated))
    }
}

impl ValidateValue<'_, rust_decimal::Decimal> for Decimal {
    fn validate(value: rust_decimal::Decimal) -> SFVResult<rust_decimal::Decimal> {
        let fraction_length = 3;

        let decimal = value.round_dp(fraction_length);
        let int_comp = decimal.trunc();
        let int_comp = int_comp
            .abs()
            .to_u64()
            .ok_or("serialize_decimal: integer component > 12 digits")?;

        if int_comp > 999_999_999_999_u64 {
            return Err("serialize_decimal: integer component > 12 digits");
        }

        Ok(decimal)
    }
}

/// Validates a bare item value and returns a new sanitized value
/// or passes back ownership of the existing value in case the input needs no change.
pub trait ValidateValue<'a, T> {
    fn validate(value: T) -> SFVResult<T>;
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
        let value = Self::validate(value)?;
        Ok(Integer(value))
    }
}

impl ValidateValue<'_, i64> for Integer {
    fn validate(value: i64) -> SFVResult<i64> {
        let (min_int, max_int) = (-999_999_999_999_999_i64, 999_999_999_999_999_i64);

        if !(min_int <= value && value <= max_int) {
            return Err("serialize_integer: integer is out of range");
        }

        Ok(value)
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
        let value = Self::validate(&value)?;
        Ok(BareItemString(value.to_owned()))
    }
}

impl TryFrom<&str> for BareItemString {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = Self::validate(value)?;
        Ok(BareItemString(value.to_owned()))
    }
}

impl<'a> ValidateValue<'a, &'a str> for BareItemString {
    fn validate(value: &'a str) -> SFVResult<&'a str> {
        if !value.is_ascii() {
            return Err("serialize_string: non-ascii character");
        }

        let vchar_or_sp = |char| char == '\x7f' || ('\x00'..='\x1f').contains(&char);
        if value.chars().any(vchar_or_sp) {
            return Err("serialize_string: not a visible character");
        }

        Ok(value)
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
        let value = Self::validate(&value)?;
        Ok(Token(value.to_owned()))
    }
}

impl TryFrom<&str> for Token {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = Self::validate(value)?;
        Ok(Token(value.to_owned()))
    }
}

impl<'a> ValidateValue<'a, &'a str> for Token {
    fn validate(value: &'a str) -> SFVResult<&'a str> {
        if !value.is_ascii() {
            return Err("serialize_string: non-ascii character");
        }

        let mut chars = value.chars();
        if let Some(char) = chars.next() {
            if !(char.is_ascii_alphabetic() || char == '*') {
                return Err("serialise_token: first character is not ALPHA or '*'");
            }
        }

        if chars
            .clone()
            .any(|c| !(utils::is_tchar(c) || c == ':' || c == '/'))
        {
            return Err("serialise_token: disallowed character");
        }

        Ok(value)
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
