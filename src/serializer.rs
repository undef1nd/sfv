use crate::utils;
use crate::{Decimal, Integer, KeyRef, RefBareItem, StringRef, TokenRef};
use std::fmt::Write as _;

#[cfg(feature = "parsed-types")]
use crate::{Dictionary, Item, List, SFVResult};

/// Serializes structured field value into String.
#[cfg(feature = "parsed-types")]
pub trait SerializeValue {
    /// Serializes structured field value into String.
    /// # Examples
    /// ```
    /// # use sfv::{Parser, SerializeValue};
    /// # fn main() -> Result<(), sfv::Error> {
    /// let parsed_list_field = Parser::from_str(r#" "london",   "berlin" "#).parse_list()?;
    ///
    /// assert_eq!(
    ///     parsed_list_field.serialize_value()?,
    ///     r#""london", "berlin""#
    /// );
    /// # Ok(())
    /// # }
    /// ```
    fn serialize_value(&self) -> SFVResult<String>;
}

#[cfg(feature = "parsed-types")]
impl SerializeValue for Dictionary {
    fn serialize_value(&self) -> SFVResult<String> {
        let mut ser = crate::RefDictSerializer::new();
        ser.members(self);
        ser.finish()
    }
}

#[cfg(feature = "parsed-types")]
impl SerializeValue for List {
    fn serialize_value(&self) -> SFVResult<String> {
        let mut ser = crate::RefListSerializer::new();
        ser.members(self);
        ser.finish()
    }
}

#[cfg(feature = "parsed-types")]
impl SerializeValue for Item {
    fn serialize_value(&self) -> SFVResult<String> {
        Ok(crate::RefItemSerializer::new()
            .bare_item(&self.bare_item)
            .parameters(&self.params)
            .finish())
    }
}

/// Container serialization functions
pub(crate) struct Serializer;

impl Serializer {
    pub(crate) fn serialize_bare_item<'b>(value: impl Into<RefBareItem<'b>>, output: &mut String) {
        // https://httpwg.org/specs/rfc8941.html#ser-bare-item

        match value.into() {
            RefBareItem::Boolean(value) => Self::serialize_bool(value, output),
            RefBareItem::String(value) => Self::serialize_string(value, output),
            RefBareItem::ByteSeq(value) => Self::serialize_byte_sequence(value, output),
            RefBareItem::Token(value) => Self::serialize_token(value, output),
            RefBareItem::Integer(value) => Self::serialize_integer(value, output),
            RefBareItem::Decimal(value) => Self::serialize_decimal(value, output),
        }
    }

    pub(crate) fn serialize_parameter<'b>(
        name: &KeyRef,
        value: impl Into<RefBareItem<'b>>,
        output: &mut String,
    ) {
        // https://httpwg.org/specs/rfc8941.html#ser-params
        output.push(';');
        Self::serialize_key(name, output);

        let value = value.into();
        if value != RefBareItem::Boolean(true) {
            output.push('=');
            Self::serialize_bare_item(value, output);
        }
    }

    pub(crate) fn serialize_key(input_key: &KeyRef, output: &mut String) {
        // https://httpwg.org/specs/rfc8941.html#ser-key

        output.push_str(input_key.as_str());
    }

    pub(crate) fn serialize_integer(value: Integer, output: &mut String) {
        //https://httpwg.org/specs/rfc8941.html#ser-integer

        write!(output, "{}", value).unwrap();
    }

    pub(crate) fn serialize_decimal(value: Decimal, output: &mut String) {
        // https://httpwg.org/specs/rfc8941.html#ser-decimal

        write!(output, "{}", value).unwrap();
    }

    pub(crate) fn serialize_string(value: &StringRef, output: &mut String) {
        // https://httpwg.org/specs/rfc8941.html#ser-string

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
        // https://httpwg.org/specs/rfc8941.html#ser-token

        output.push_str(value.as_str());
    }

    pub(crate) fn serialize_byte_sequence(value: &[u8], output: &mut String) {
        // https://httpwg.org/specs/rfc8941.html#ser-binary

        output.push(':');
        base64::Engine::encode_string(&utils::BASE64, value, output);
        output.push(':');
    }

    pub(crate) fn serialize_bool(value: bool, output: &mut String) {
        // https://httpwg.org/specs/rfc8941.html#ser-boolean

        output.push_str(if value { "?1" } else { "?0" });
    }
}
