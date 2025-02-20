use crate::utils;
use crate::{
    BareItem, Decimal, Dictionary, Error, InnerList, Item, List, ListEntry, Parameters,
    RefBareItem, SFVResult,
};
use std::fmt::Write as _;

/// Serializes structured field value into String.
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

impl SerializeValue for Dictionary {
    fn serialize_value(&self) -> SFVResult<String> {
        let mut output = String::new();
        Serializer::serialize_dict(self, &mut output)?;
        Ok(output)
    }
}

impl SerializeValue for List {
    fn serialize_value(&self) -> SFVResult<String> {
        let mut output = String::new();
        Serializer::serialize_list(self, &mut output)?;
        Ok(output)
    }
}

impl SerializeValue for Item {
    fn serialize_value(&self) -> SFVResult<String> {
        let mut output = String::new();
        Serializer::serialize_item(self, &mut output)?;
        Ok(output)
    }
}

/// Container serialization functions
pub(crate) struct Serializer;

impl Serializer {
    pub(crate) fn serialize_item(input_item: &Item, output: &mut String) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#ser-item

        Self::serialize_bare_item(&input_item.bare_item, output)?;
        Self::serialize_parameters(&input_item.params, output)
    }

    pub(crate) fn serialize_list(input_list: &List, output: &mut String) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#ser-list
        if input_list.is_empty() {
            return Err(Error::new(
                "serialize_list: serializing empty field is not allowed",
            ));
        }

        for (idx, member) in input_list.iter().enumerate() {
            match member {
                ListEntry::Item(item) => {
                    Self::serialize_item(item, output)?;
                }
                ListEntry::InnerList(inner_list) => {
                    Self::serialize_inner_list(inner_list, output)?;
                }
            }

            // If more items remain in input_list:
            //      Append “,” to output.
            //      Append a single SP to output.
            if idx < input_list.len() - 1 {
                output.push_str(", ");
            }
        }
        Ok(())
    }

    pub(crate) fn serialize_dict(input_dict: &Dictionary, output: &mut String) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#ser-dictionary
        if input_dict.is_empty() {
            return Err(Error::new(
                "serialize_dictionary: serializing empty field is not allowed",
            ));
        }

        for (idx, (member_name, member_value)) in input_dict.iter().enumerate() {
            Serializer::serialize_key(member_name, output)?;

            match member_value {
                ListEntry::Item(item) => {
                    // If dict member is boolean true, no need to serialize it: only its params must be serialized
                    // Otherwise serialize entire item with its params
                    if item.bare_item == BareItem::Boolean(true) {
                        Self::serialize_parameters(&item.params, output)?;
                    } else {
                        output.push('=');
                        Self::serialize_item(item, output)?;
                    }
                }
                ListEntry::InnerList(inner_list) => {
                    output.push('=');
                    Self::serialize_inner_list(inner_list, output)?;
                }
            }

            // If more items remain in input_dictionary:
            //      Append “,” to output.
            //      Append a single SP to output.
            if idx < input_dict.len() - 1 {
                output.push_str(", ");
            }
        }
        Ok(())
    }

    fn serialize_inner_list(input_inner_list: &InnerList, output: &mut String) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#ser-innerlist

        let items = &input_inner_list.items;
        let inner_list_parameters = &input_inner_list.params;

        output.push('(');
        for (idx, item) in items.iter().enumerate() {
            Self::serialize_item(item, output)?;

            // If more values remain in inner_list, append a single SP to output
            if idx < items.len() - 1 {
                output.push(' ');
            }
        }
        output.push(')');
        Self::serialize_parameters(inner_list_parameters, output)
    }

    pub(crate) fn serialize_bare_item<'b>(
        value: impl Into<RefBareItem<'b>>,
        output: &mut String,
    ) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#ser-bare-item

        match value.into() {
            RefBareItem::Boolean(value) => Self::serialize_bool(value, output),
            RefBareItem::String(value) => Self::serialize_string(value, output)?,
            RefBareItem::ByteSeq(value) => Self::serialize_byte_sequence(value, output),
            RefBareItem::Token(value) => Self::serialize_token(value, output)?,
            RefBareItem::Integer(value) => Self::serialize_integer(value, output)?,
            RefBareItem::Decimal(value) => Self::serialize_decimal(value, output)?,
        };
        Ok(())
    }

    pub(crate) fn serialize_parameters(
        input_params: &Parameters,
        output: &mut String,
    ) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#ser-params

        for (param_name, param_value) in input_params {
            Self::serialize_parameter(param_name, param_value, output)?;
        }
        Ok(())
    }

    pub(crate) fn serialize_parameter<'b>(
        name: &str,
        value: impl Into<RefBareItem<'b>>,
        output: &mut String,
    ) -> SFVResult<()> {
        output.push(';');
        Self::serialize_key(name, output)?;

        let value = value.into();
        if value != RefBareItem::Boolean(true) {
            output.push('=');
            Self::serialize_bare_item(value, output)?;
        }
        Ok(())
    }

    pub(crate) fn serialize_key(input_key: &str, output: &mut String) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#ser-key

        let mut bytes = input_key.bytes();

        match bytes.next() {
            None => return Err(Error::new("serialize_key: key is empty")),
            Some(c) => {
                if !utils::is_allowed_start_key_char(c) {
                    return Err(Error::new(
                        "serialize_key: first character is not lcalpha or '*'",
                    ));
                }
            }
        }

        if bytes.any(|c| !utils::is_allowed_inner_key_char(c)) {
            return Err(Error::new("serialize_key: disallowed character"));
        }

        output.push_str(input_key);
        Ok(())
    }

    pub(crate) fn serialize_integer(value: i64, output: &mut String) -> SFVResult<()> {
        //https://httpwg.org/specs/rfc8941.html#ser-integer

        let (min_int, max_int) = (-999_999_999_999_999_i64, 999_999_999_999_999_i64);
        if !(min_int <= value && value <= max_int) {
            return Err(Error::new("serialize_integer: integer is out of range"));
        }
        write!(output, "{}", value).unwrap();
        Ok(())
    }

    pub(crate) fn serialize_decimal(value: Decimal, output: &mut String) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#ser-decimal

        let fraction_length = 3;

        let decimal = value.round_dp(fraction_length).normalize();
        let int_comp = decimal.trunc();
        let fract_comp = decimal.fract();

        if int_comp.abs() > Decimal::from(999_999_999_999_i64) {
            return Err(Error::new(
                "serialize_decimal: integer component > 12 digits",
            ));
        }

        if fract_comp.is_zero() {
            write!(output, "{}.0", int_comp).unwrap();
        } else {
            write!(output, "{}", decimal).unwrap();
        }

        Ok(())
    }

    pub(crate) fn serialize_string(value: &str, output: &mut String) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#ser-integer

        if !value.is_ascii() {
            return Err(Error::new("serialize_string: non-ascii character"));
        }

        let vchar_or_sp = |char| char == '\x7f' || ('\x00'..='\x1f').contains(&char);
        if value.chars().any(vchar_or_sp) {
            return Err(Error::new("serialize_string: not a visible character"));
        }

        output.push('"');
        for char in value.chars() {
            if char == '\\' || char == '"' {
                output.push('\\');
            }
            output.push(char);
        }
        output.push('"');

        Ok(())
    }

    pub(crate) fn serialize_token(value: &str, output: &mut String) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#ser-token

        if !value.is_ascii() {
            return Err(Error::new("serialize_token: non-ascii character"));
        }

        let mut bytes = value.bytes();

        match bytes.next() {
            None => return Err(Error::new("serialize_token: token is empty")),
            Some(c) => {
                if !utils::is_allowed_start_token_char(c) {
                    return Err(Error::new(
                        "serialize_token: first character is not ALPHA or '*'",
                    ));
                }
            }
        }

        if bytes.any(|c| !utils::is_allowed_inner_token_char(c)) {
            return Err(Error::new("serialize_token: disallowed character"));
        }

        output.push_str(value);
        Ok(())
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
