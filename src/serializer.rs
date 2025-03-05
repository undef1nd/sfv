use crate::utils;
use crate::{
    BareItem, Decimal, Dictionary, Error, InnerList, Integer, Item, KeyRef, List, ListEntry,
    Parameters, RefBareItem, SFVResult, StringRef, TokenRef,
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
        Serializer::serialize_item(self, &mut output);
        Ok(output)
    }
}

/// Container serialization functions
pub(crate) struct Serializer;

impl Serializer {
    pub(crate) fn serialize_item(input_item: &Item, output: &mut String) {
        // https://httpwg.org/specs/rfc8941.html#ser-item

        Self::serialize_bare_item(&input_item.bare_item, output);
        Self::serialize_parameters(&input_item.params, output);
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
                    Self::serialize_item(item, output);
                }
                ListEntry::InnerList(inner_list) => {
                    Self::serialize_inner_list(inner_list, output);
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
            Serializer::serialize_key(member_name, output);

            match member_value {
                ListEntry::Item(item) => {
                    // If dict member is boolean true, no need to serialize it: only its params must be serialized
                    // Otherwise serialize entire item with its params
                    if item.bare_item == BareItem::Boolean(true) {
                        Self::serialize_parameters(&item.params, output);
                    } else {
                        output.push('=');
                        Self::serialize_item(item, output);
                    }
                }
                ListEntry::InnerList(inner_list) => {
                    output.push('=');
                    Self::serialize_inner_list(inner_list, output);
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

    fn serialize_inner_list(input_inner_list: &InnerList, output: &mut String) {
        // https://httpwg.org/specs/rfc8941.html#ser-innerlist

        let items = &input_inner_list.items;
        let inner_list_parameters = &input_inner_list.params;

        output.push('(');
        for (idx, item) in items.iter().enumerate() {
            Self::serialize_item(item, output);

            // If more values remain in inner_list, append a single SP to output
            if idx < items.len() - 1 {
                output.push(' ');
            }
        }
        output.push(')');
        Self::serialize_parameters(inner_list_parameters, output);
    }

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

    pub(crate) fn serialize_parameters(input_params: &Parameters, output: &mut String) {
        // https://httpwg.org/specs/rfc8941.html#ser-params

        for (param_name, param_value) in input_params {
            Self::serialize_parameter(param_name, param_value, output);
        }
    }

    pub(crate) fn serialize_parameter<'b>(
        name: &KeyRef,
        value: impl Into<RefBareItem<'b>>,
        output: &mut String,
    ) {
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
