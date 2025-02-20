use crate::utils;
use crate::{
    BareItem, Decimal, Dictionary, InnerList, Item, List, ListEntry, Num, Parameters, SFVResult,
};

trait ParseValue {
    fn parse(parser: &mut Parser) -> SFVResult<Self>
    where
        Self: Sized;
}

/// If structured field value of List or Dictionary type is split into multiple lines,
/// allows to parse more lines and merge them into already existing structure field value.
pub trait ParseMore {
    /// If structured field value is split across lines,
    /// parses and merges next line into a single structured field value.
    /// # Examples
    /// ```
    /// # use sfv::{Parser, SerializeValue, ParseMore};
    ///
    /// let mut list_field = Parser::from_str("11, (12 13)").parse_list().unwrap();
    /// list_field.parse_more("\"foo\",        \"bar\"".as_bytes()).unwrap();
    ///
    /// assert_eq!(list_field.serialize_value().unwrap(), "11, (12 13), \"foo\", \"bar\"");
    fn parse_more(&mut self, input_bytes: &[u8]) -> SFVResult<()>
    where
        Self: Sized;
}

impl ParseValue for Item {
    fn parse(parser: &mut Parser) -> SFVResult<Item> {
        // https://httpwg.org/specs/rfc8941.html#parse-item
        let bare_item = parser.parse_bare_item()?;
        let params = parser.parse_parameters()?;

        Ok(Item { bare_item, params })
    }
}

impl ParseValue for List {
    fn parse(parser: &mut Parser) -> SFVResult<List> {
        // https://httpwg.org/specs/rfc8941.html#parse-list
        // List represents an array of (item_or_inner_list, parameters)

        let mut members = vec![];

        while parser.input.peek().is_some() {
            members.push(parser.parse_list_entry()?);

            parser.consume_ows_chars();

            if parser.input.peek().is_none() {
                return Ok(members);
            }

            if let Some(c) = parser.input.next() {
                if c != b',' {
                    return Err("parse_list: trailing characters after list member");
                }
            }

            parser.consume_ows_chars();

            if parser.input.peek().is_none() {
                return Err("parse_list: trailing comma");
            }
        }

        Ok(members)
    }
}

impl ParseValue for Dictionary {
    fn parse(parser: &mut Parser) -> SFVResult<Dictionary> {
        let mut dict = Dictionary::new();

        while parser.input.peek().is_some() {
            let this_key = parser.parse_key()?;

            if let Some(b'=') = parser.input.peek() {
                parser.input.next();
                let member = parser.parse_list_entry()?;
                dict.insert(this_key, member);
            } else {
                let value = true;
                let params = parser.parse_parameters()?;
                let member = Item {
                    bare_item: BareItem::Boolean(value),
                    params,
                };
                dict.insert(this_key, member.into());
            }

            parser.consume_ows_chars();

            if parser.input.peek().is_none() {
                return Ok(dict);
            }

            if let Some(c) = parser.input.next() {
                if c != b',' {
                    return Err("parse_dict: trailing characters after dictionary member");
                }
            }

            parser.consume_ows_chars();

            if parser.input.peek().is_none() {
                return Err("parse_dict: trailing comma");
            }
        }
        Ok(dict)
    }
}

impl ParseMore for List {
    fn parse_more(&mut self, input_bytes: &[u8]) -> SFVResult<()> {
        let parsed_list = Parser::from_bytes(input_bytes).parse_list()?;
        self.extend(parsed_list);
        Ok(())
    }
}

impl ParseMore for Dictionary {
    fn parse_more(&mut self, input_bytes: &[u8]) -> SFVResult<()> {
        let parsed_dict = Parser::from_bytes(input_bytes).parse_dictionary()?;
        self.extend(parsed_dict);
        Ok(())
    }
}

/// Exposes methods for parsing input into structured field value.
pub struct Parser<'a> {
    input: std::iter::Peekable<std::iter::Copied<std::slice::Iter<'a, u8>>>,
}

impl<'a> Parser<'a> {
    pub fn from_bytes(input: &'a [u8]) -> Self {
        Self {
            input: input.iter().copied().peekable(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'a str) -> Self {
        Self::from_bytes(input.as_bytes())
    }

    /// Parses input into structured field value of Dictionary type
    pub fn parse_dictionary(self) -> SFVResult<Dictionary> {
        self.parse()
    }

    /// Parses input into structured field value of List type
    pub fn parse_list(self) -> SFVResult<List> {
        self.parse()
    }

    /// Parses input into structured field value of Item type
    pub fn parse_item(self) -> SFVResult<Item> {
        self.parse()
    }

    // Generic parse method for checking input before parsing
    // and handling trailing text error
    fn parse<T: ParseValue>(mut self) -> SFVResult<T> {
        // https://httpwg.org/specs/rfc8941.html#text-parse

        self.consume_sp_chars();

        let output = T::parse(&mut self)?;

        self.consume_sp_chars();

        if self.input.peek().is_some() {
            return Err("parse: trailing characters after parsed value");
        };
        Ok(output)
    }

    fn parse_list_entry(&mut self) -> SFVResult<ListEntry> {
        // https://httpwg.org/specs/rfc8941.html#parse-item-or-list
        // ListEntry represents a tuple (item_or_inner_list, parameters)

        match self.input.peek() {
            Some(b'(') => {
                let parsed = self.parse_inner_list()?;
                Ok(ListEntry::InnerList(parsed))
            }
            _ => {
                let parsed = Item::parse(self)?;
                Ok(ListEntry::Item(parsed))
            }
        }
    }

    pub(crate) fn parse_inner_list(&mut self) -> SFVResult<InnerList> {
        // https://httpwg.org/specs/rfc8941.html#parse-innerlist

        if Some(b'(') != self.input.next() {
            return Err("parse_inner_list: input does not start with '('");
        }

        let mut inner_list = Vec::new();
        while self.input.peek().is_some() {
            self.consume_sp_chars();

            if Some(&b')') == self.input.peek() {
                self.input.next();
                let params = self.parse_parameters()?;
                return Ok(InnerList {
                    items: inner_list,
                    params,
                });
            }

            let parsed_item = Item::parse(self)?;
            inner_list.push(parsed_item);

            if let Some(c) = self.input.peek() {
                if c != &b' ' && c != &b')' {
                    return Err("parse_inner_list: bad delimitation");
                }
            }
        }

        Err("parse_inner_list: the end of the inner list was not found")
    }

    pub(crate) fn parse_bare_item(&mut self) -> SFVResult<BareItem> {
        // https://httpwg.org/specs/rfc8941.html#parse-bare-item
        if self.input.peek().is_none() {
            return Err("parse_bare_item: empty item");
        }

        match self.input.peek() {
            Some(b'?') => Ok(BareItem::Boolean(self.parse_bool()?)),
            Some(b'"') => Ok(BareItem::String(self.parse_string()?)),
            Some(b':') => Ok(BareItem::ByteSeq(self.parse_byte_sequence()?)),
            Some(&c) if c == b'*' || c.is_ascii_alphabetic() => {
                Ok(BareItem::Token(self.parse_token()?))
            }
            Some(&c) if c == b'-' || c.is_ascii_digit() => match self.parse_number()? {
                Num::Decimal(val) => Ok(BareItem::Decimal(val)),
                Num::Integer(val) => Ok(BareItem::Integer(val)),
            },
            _ => Err("parse_bare_item: item type can't be identified"),
        }
    }

    pub(crate) fn parse_bool(&mut self) -> SFVResult<bool> {
        // https://httpwg.org/specs/rfc8941.html#parse-boolean

        if self.input.next() != Some(b'?') {
            return Err("parse_bool: first character is not '?'");
        }

        match self.input.next() {
            Some(b'0') => Ok(false),
            Some(b'1') => Ok(true),
            _ => Err("parse_bool: invalid variant"),
        }
    }

    pub(crate) fn parse_string(&mut self) -> SFVResult<String> {
        // https://httpwg.org/specs/rfc8941.html#parse-string

        if self.input.next() != Some(b'"') {
            return Err("parse_string: first character is not '\"'");
        }

        let mut output_string = String::from("");
        while let Some(curr_char) = self.input.next() {
            match curr_char {
                b'"' => return Ok(output_string),
                0x00..=0x1f | 0x7f..=0xff => return Err("parse_string: invalid string character"),
                b'\\' => match self.input.next() {
                    Some(c @ b'\\' | c @ b'\"') => {
                        output_string.push(c as char);
                    }
                    None => return Err("parse_string: last input character is '\\'"),
                    _ => return Err("parse_string: disallowed character after '\\'"),
                },
                _ => output_string.push(curr_char as char),
            }
        }
        Err("parse_string: no closing '\"'")
    }

    pub(crate) fn parse_token(&mut self) -> SFVResult<String> {
        // https://httpwg.org/specs/rfc8941.html#parse-token

        if let Some(first_char) = self.input.peek() {
            if !utils::is_allowed_start_token_char(*first_char) {
                return Err("parse_token: first character is not ALPHA or '*'");
            }
        } else {
            return Err("parse_token: empty input string");
        }

        let mut output_string = String::from("");
        while let Some(curr_char) = self.input.peek() {
            if !utils::is_allowed_inner_token_char(*curr_char) {
                return Ok(output_string);
            }

            match self.input.next() {
                Some(c) => output_string.push(c as char),
                None => return Err("parse_token: end of the string"),
            }
        }
        Ok(output_string)
    }

    pub(crate) fn parse_byte_sequence(&mut self) -> SFVResult<Vec<u8>> {
        // https://httpwg.org/specs/rfc8941.html#parse-binary

        if self.input.next() != Some(b':') {
            return Err("parse_byte_seq: first char is not ':'");
        }

        let mut b64_content = vec![];
        loop {
            match self.input.next() {
                Some(b':') => break,
                Some(c) => b64_content.push(c),
                None => return Err("parse_byte_seq: no closing ':'"),
            }
        }

        match base64::Engine::decode(&utils::BASE64, b64_content) {
            Ok(content) => Ok(content),
            Err(_) => Err("parse_byte_seq: decoding error"),
        }
    }

    pub(crate) fn parse_number(&mut self) -> SFVResult<Num> {
        // https://httpwg.org/specs/rfc8941.html#parse-number

        fn char_to_i64(c: u8) -> i64 {
            (c - b'0') as i64
        }

        let sign = if let Some(b'-') = self.input.peek() {
            self.input.next();
            -1
        } else {
            1
        };

        let mut magnitude = match self.input.peek() {
            Some(&c @ b'0'..=b'9') => {
                self.input.next();
                char_to_i64(c)
            }
            _ => return Err("parse_number: expected digit"),
        };

        let mut digits = 1;

        loop {
            match self.input.peek() {
                Some(b'.') => {
                    if digits > 12 {
                        return Err("parse_number: too many digits before decimal point");
                    }
                    self.input.next();
                    break;
                }
                Some(&c @ b'0'..=b'9') => {
                    digits += 1;
                    if digits > 15 {
                        return Err("parse_number: too many digits");
                    }
                    self.input.next();
                    magnitude = magnitude * 10 + char_to_i64(c);
                }
                _ => return Ok(Num::Integer(sign * magnitude)),
            }
        }

        digits = 0;

        while let Some(&c @ b'0'..=b'9') = self.input.peek() {
            if digits == 3 {
                return Err("parse_number: too many digits after decimal point");
            }

            self.input.next();
            magnitude = magnitude * 10 + char_to_i64(c);
            digits += 1;
        }

        if digits == 0 {
            Err("parse_number: trailing decimal point")
        } else {
            Ok(Num::Decimal(Decimal::from_i128_with_scale(
                (sign * magnitude) as i128,
                digits,
            )))
        }
    }

    pub(crate) fn parse_parameters(&mut self) -> SFVResult<Parameters> {
        // https://httpwg.org/specs/rfc8941.html#parse-param

        let mut params = Parameters::new();

        while let Some(curr_char) = self.input.peek() {
            if curr_char == &b';' {
                self.input.next();
            } else {
                break;
            }

            self.consume_sp_chars();

            let param_name = self.parse_key()?;
            let param_value = match self.input.peek() {
                Some(b'=') => {
                    self.input.next();
                    self.parse_bare_item()?
                }
                _ => BareItem::Boolean(true),
            };
            params.insert(param_name, param_value);
        }

        // If parameters already contains a name param_name (comparing character-for-character), overwrite its value.
        // Note that when duplicate Parameter keys are encountered, this has the effect of ignoring all but the last instance.
        Ok(params)
    }

    pub(crate) fn parse_key(&mut self) -> SFVResult<String> {
        match self.input.peek() {
            Some(c) if c == &b'*' || c.is_ascii_lowercase() => (),
            _ => return Err("parse_key: first character is not lcalpha or '*'"),
        }

        let mut output = String::new();
        while let Some(curr_char) = self.input.peek() {
            if !curr_char.is_ascii_lowercase()
                && !curr_char.is_ascii_digit()
                && !b"_-*.".contains(curr_char)
            {
                return Ok(output);
            }

            output.push(*curr_char as char);
            self.input.next();
        }
        Ok(output)
    }

    fn consume_ows_chars(&mut self) {
        while let Some(b' ' | b'\t') = self.input.peek() {
            self.input.next();
        }
    }

    fn consume_sp_chars(&mut self) {
        while let Some(b' ') = self.input.peek() {
            self.input.next();
        }
    }

    #[cfg(test)]
    pub(crate) fn remaining(self) -> Vec<u8> {
        self.input.collect()
    }
}
