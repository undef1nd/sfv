use std::fmt::Write as _;

use crate::utils;
#[cfg(feature = "parsed-types")]
use crate::{private::Sealed, Dictionary, Item, List};
use crate::{Date, Decimal, Integer, KeyRef, RefBareItem, StringRef, TokenRef};

/// Serializes a structured field value into a string.
///
/// Note: The serialization conforms to [RFC 9651], meaning that
/// [`Dates`][crate::Date] and [`Display Strings`][RefBareItem::DisplayString],
/// which cause parsing errors under [RFC 8941], will be serialized
/// unconditionally. The consumer of this API is responsible for determining
/// whether it is valid to serialize these bare items for any specific header.
///
/// [RFC 8941]: <https://httpwg.org/specs/rfc8941.html>
/// [RFC 9651]: <https://httpwg.org/specs/rfc9651.html>
///
/// Use [`crate::ItemSerializer`], [`crate::ListSerializer`], or
/// [`crate::DictSerializer`] to serialize components incrementally without
/// having to create an [`Item`], [`List`], or [`Dictionary`].
#[cfg(feature = "parsed-types")]
pub trait SerializeValue: Sealed {
    /// The result of serializing the value into a string.
    ///
    /// [`Item`] serialization is infallible; [`List`] and [`Dictionary`]
    /// serialization is not.
    type Result: Into<Option<String>>;

    /// Serializes a structured field value into a string.
    ///
    /// # Examples
    /// ```
    /// # use sfv::{Parser, SerializeValue};
    /// # fn main() -> Result<(), sfv::Error> {
    /// let parsed_list_field = Parser::new(r#" "london",   "berlin" "#).parse_list()?;
    ///
    /// assert_eq!(
    ///     parsed_list_field.serialize_value().as_deref(),
    ///     Some(r#""london", "berlin""#),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn serialize_value(&self) -> Self::Result;
}

#[cfg(feature = "parsed-types")]
impl Sealed for Dictionary {}

#[cfg(feature = "parsed-types")]
impl SerializeValue for Dictionary {
    type Result = Option<String>;

    fn serialize_value(&self) -> Option<String> {
        let mut ser = crate::DictSerializer::new();
        ser.members(self);
        ser.finish()
    }
}

#[cfg(feature = "parsed-types")]
impl Sealed for List {}

#[cfg(feature = "parsed-types")]
impl SerializeValue for List {
    type Result = Option<String>;

    fn serialize_value(&self) -> Option<String> {
        let mut ser = crate::ListSerializer::new();
        ser.members(self);
        ser.finish()
    }
}

#[cfg(feature = "parsed-types")]
impl Sealed for Item {}

#[cfg(feature = "parsed-types")]
impl SerializeValue for Item {
    type Result = String;

    fn serialize_value(&self) -> String {
        crate::ItemSerializer::new()
            .bare_item(&self.bare_item)
            .parameters(&self.params)
            .finish()
    }
}

pub(crate) struct Serializer;

impl Serializer {
    pub(crate) fn serialize_bare_item<'b>(value: impl Into<RefBareItem<'b>>, output: &mut String) {
        // https://httpwg.org/specs/rfc9651.html#ser-bare-item

        match value.into() {
            RefBareItem::Boolean(value) => Self::serialize_bool(value, output),
            RefBareItem::String(value) => Self::serialize_string(value, output),
            RefBareItem::ByteSequence(value) => Self::serialize_byte_sequence(value, output),
            RefBareItem::Token(value) => Self::serialize_token(value, output),
            RefBareItem::Integer(value) => Self::serialize_integer(value, output),
            RefBareItem::Decimal(value) => Self::serialize_decimal(value, output),
            RefBareItem::Date(value) => Self::serialize_date(value, output),
            RefBareItem::DisplayString(value) => Self::serialize_display_string(value, output),
        }
    }

    pub(crate) fn serialize_parameter<'b>(
        name: &KeyRef,
        value: impl Into<RefBareItem<'b>>,
        output: &mut String,
    ) {
        // https://httpwg.org/specs/rfc9651.html#ser-params
        output.push(';');
        Self::serialize_key(name, output);

        let value = value.into();
        if value != RefBareItem::Boolean(true) {
            output.push('=');
            Self::serialize_bare_item(value, output);
        }
    }

    pub(crate) fn serialize_key(input_key: &KeyRef, output: &mut String) {
        // https://httpwg.org/specs/rfc9651.html#ser-key

        output.push_str(input_key.as_str());
    }

    pub(crate) fn serialize_integer(value: Integer, output: &mut String) {
        //https://httpwg.org/specs/rfc9651.html#ser-integer

        write!(output, "{}", value).unwrap();
    }

    pub(crate) fn serialize_decimal(value: Decimal, output: &mut String) {
        // https://httpwg.org/specs/rfc9651.html#ser-decimal

        write!(output, "{}", value).unwrap();
    }

    pub(crate) fn serialize_string(value: &StringRef, output: &mut String) {
        // https://httpwg.org/specs/rfc9651.html#ser-string

        output.push('"');
        for char in value.as_str().chars() {
            if char == '\\' || char == '"' {
                output.push('\\');
            }
            output.push(char);
        }
        output.push('"');
    }

    pub(crate) fn serialize_token(value: &TokenRef, output: &mut String) {
        // https://httpwg.org/specs/rfc9651.html#ser-token

        output.push_str(value.as_str());
    }

    pub(crate) fn serialize_byte_sequence(value: &[u8], output: &mut String) {
        // https://httpwg.org/specs/rfc9651.html#ser-binary

        output.push(':');
        base64::Engine::encode_string(&utils::BASE64, value, output);
        output.push(':');
    }

    pub(crate) fn serialize_bool(value: bool, output: &mut String) {
        // https://httpwg.org/specs/rfc9651.html#ser-boolean

        output.push_str(if value { "?1" } else { "?0" });
    }

    pub(crate) fn serialize_date(value: Date, output: &mut String) {
        // https://httpwg.org/specs/rfc9651.html#ser-date

        write!(output, "{}", value).unwrap();
    }

    pub(crate) fn serialize_display_string(value: &str, output: &mut String) {
        // https://httpwg.org/specs/rfc9651.html#ser-display

        output.push_str(r#"%""#);
        for c in value.bytes() {
            match c {
                b'%' | b'"' | 0x00..=0x1f | 0x7f..=0xff => {
                    output.push('%');
                    output.push(char::from_digit((c as u32 >> 4) & 0xf, 16).unwrap());
                    output.push(char::from_digit(c as u32 & 0xf, 16).unwrap());
                }
                _ => output.push(c as char),
            }
        }
        output.push('"');
    }
}
