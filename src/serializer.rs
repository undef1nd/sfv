use crate::utils;
use crate::{
    Date, Decimal, Error, Format, Integer, KeyRef, RefBareItem, SFVResult, StringRef, TokenRef,
};
use std::fmt::Write as _;

#[cfg(feature = "parsed-types")]
use crate::{Dictionary, Item, List};

/// Serializes a structured field value into a string using [`Format::Rfc9651`].
///
/// Use [`ItemSerializer::with_format`][`crate::ItemSerializer::with_format`],
/// [`DictSerializer::with_format`][`crate::DictSerializer::with_format`], or
/// [`ListSerializer::with_format`][`crate::ListSerializer::with_format`] to use a different format.
#[cfg(feature = "parsed-types")]
pub trait SerializeValue {
    /// Serializes a structured field value into a string using
    /// [`Format::Rfc9651`].
    ///
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
        let mut ser = crate::DictSerializer::new();
        ser.members(self)?;
        ser.finish()
    }
}

#[cfg(feature = "parsed-types")]
impl SerializeValue for List {
    fn serialize_value(&self) -> SFVResult<String> {
        let mut ser = crate::ListSerializer::new();
        ser.members(self)?;
        ser.finish()
    }
}

#[cfg(feature = "parsed-types")]
impl SerializeValue for Item {
    fn serialize_value(&self) -> SFVResult<String> {
        Ok(crate::ItemSerializer::new()
            .bare_item(&self.bare_item)?
            .parameters(&self.params)?
            .finish())
    }
}

impl Format {
    pub(crate) fn serialize_bare_item<'b>(
        &self,
        value: impl Into<RefBareItem<'b>>,
        output: &mut String,
    ) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc9651.html#ser-bare-item

        Ok(match value.into() {
            RefBareItem::Boolean(value) => Self::serialize_bool(value, output),
            RefBareItem::String(value) => Self::serialize_string(value, output),
            RefBareItem::ByteSeq(value) => Self::serialize_byte_sequence(value, output),
            RefBareItem::Token(value) => Self::serialize_token(value, output),
            RefBareItem::Integer(value) => Self::serialize_integer(value, output),
            RefBareItem::Decimal(value) => Self::serialize_decimal(value, output),
            RefBareItem::Date(value) => self.serialize_date(value, output)?,
            RefBareItem::DisplayString(value) => self.serialize_display_string(value, output)?,
        })
    }

    pub(crate) fn serialize_parameter<'b>(
        &self,
        name: &KeyRef,
        value: impl Into<RefBareItem<'b>>,
        output: &mut String,
    ) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc9651.html#ser-params
        output.push(';');
        Self::serialize_key(name, output);

        let value = value.into();
        if value != RefBareItem::Boolean(true) {
            output.push('=');
            self.serialize_bare_item(value, output)?;
        }
        Ok(())
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

    pub(crate) fn serialize_date(&self, value: Date, output: &mut String) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc9651.html#ser-date

        match self {
            Self::Rfc8941 => Err(Error::date_unsupported()),
            Self::Rfc9651 => {
                write!(output, "{}", value).unwrap();
                Ok(())
            }
        }
    }

    pub(crate) fn serialize_display_string(
        &self,
        value: &str,
        output: &mut String,
    ) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc9651.html#ser-display

        match self {
            Self::Rfc8941 => return Err(Error::display_string_unsupported()),
            Self::Rfc9651 => {}
        }

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
        Ok(())
    }
}
