use crate::utils;
use crate::{
    BareItem, Decimal, Dictionary, Error, InnerList, Integer, Item, Key, List, ListEntry, Num,
    Parameters, SFVResult, String, Token,
};

use std::convert::TryFrom;
use std::string::String as StdString;

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
    /// # use sfv::{ParseMore, Parser, SerializeValue};
    /// # fn main() -> Result<(), sfv::Error> {
    /// let mut list_field = Parser::from_str("11, (12 13)").parse_list()?;
    ///
    /// list_field.parse_more(r#""foo",        "bar""#.as_bytes())?;
    ///
    /// assert_eq!(
    ///     list_field.serialize_value()?,
    ///     r#"11, (12 13), "foo", "bar""#,
    /// );
    /// # Ok(())
    /// # }
    /// ```
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

        while parser.peek().is_some() {
            members.push(parser.parse_list_entry()?);

            parser.consume_ows_chars();

            if parser.peek().is_none() {
                return Ok(members);
            }

            let comma_index = parser.index;

            if let Some(c) = parser.peek() {
                if c != b',' {
                    return parser.error("trailing characters after list member");
                }
                parser.next();
            }

            parser.consume_ows_chars();

            if parser.peek().is_none() {
                // Report the error at the position of the comma itself, rather
                // than at the end of input.
                return Err(Error::with_index("trailing comma", comma_index));
            }
        }

        Ok(members)
    }
}

impl ParseValue for Dictionary {
    fn parse(parser: &mut Parser) -> SFVResult<Dictionary> {
        let mut dict = Dictionary::new();

        while parser.peek().is_some() {
            let this_key = parser.parse_key()?;

            if let Some(b'=') = parser.peek() {
                parser.next();
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

            if parser.peek().is_none() {
                return Ok(dict);
            }

            let comma_index = parser.index;

            if let Some(c) = parser.peek() {
                if c != b',' {
                    return parser.error("trailing characters after dictionary member");
                }
                parser.next();
            }

            parser.consume_ows_chars();

            if parser.peek().is_none() {
                // Report the error at the position of the comma itself, rather
                // than at the end of input.
                return Err(Error::with_index("trailing comma", comma_index));
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
    input: &'a [u8],
    index: usize,
}

impl<'a> Parser<'a> {
    pub fn from_bytes(input: &'a [u8]) -> Self {
        Self { input, index: 0 }
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

    fn peek(&self) -> Option<u8> {
        self.input.get(self.index).copied()
    }

    fn next(&mut self) -> Option<u8> {
        self.peek().inspect(|_| self.index += 1)
    }

    fn error<T>(&self, msg: &'static str) -> SFVResult<T> {
        Err(Error::with_index(msg, self.index))
    }

    // Generic parse method for checking input before parsing
    // and handling trailing text error
    fn parse<T: ParseValue>(mut self) -> SFVResult<T> {
        // https://httpwg.org/specs/rfc8941.html#text-parse

        self.consume_sp_chars();

        let output = T::parse(&mut self)?;

        self.consume_sp_chars();

        if self.peek().is_some() {
            self.error("trailing characters after parsed value")
        } else {
            Ok(output)
        }
    }

    fn parse_list_entry(&mut self) -> SFVResult<ListEntry> {
        // https://httpwg.org/specs/rfc8941.html#parse-item-or-list
        // ListEntry represents a tuple (item_or_inner_list, parameters)

        match self.peek() {
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

        if Some(b'(') != self.peek() {
            return self.error("expected start of inner list");
        }

        self.next();

        let mut inner_list = Vec::new();
        while self.peek().is_some() {
            self.consume_sp_chars();

            if Some(b')') == self.peek() {
                self.next();
                let params = self.parse_parameters()?;
                return Ok(InnerList {
                    items: inner_list,
                    params,
                });
            }

            let parsed_item = Item::parse(self)?;
            inner_list.push(parsed_item);

            if let Some(c) = self.peek() {
                if c != b' ' && c != b')' {
                    return self.error("expected inner list delimiter (' ' or ')')");
                }
            }
        }

        self.error("unterminated inner list")
    }

    pub(crate) fn parse_bare_item(&mut self) -> SFVResult<BareItem> {
        // https://httpwg.org/specs/rfc8941.html#parse-bare-item

        match self.peek() {
            Some(b'?') => Ok(BareItem::Boolean(self.parse_bool()?)),
            Some(b'"') => Ok(BareItem::String(self.parse_string()?)),
            Some(b':') => Ok(BareItem::ByteSeq(self.parse_byte_sequence()?)),
            Some(c) if utils::is_allowed_start_token_char(c) => {
                Ok(BareItem::Token(self.parse_token()?))
            }
            Some(c) if c == b'-' || c.is_ascii_digit() => match self.parse_number()? {
                Num::Decimal(val) => Ok(BareItem::Decimal(val)),
                Num::Integer(val) => Ok(BareItem::Integer(val)),
            },
            _ => self.error("expected start of bare item"),
        }
    }

    pub(crate) fn parse_bool(&mut self) -> SFVResult<bool> {
        // https://httpwg.org/specs/rfc8941.html#parse-boolean

        if self.peek() != Some(b'?') {
            return self.error("expected start of boolean ('?')");
        }

        self.next();

        match self.peek() {
            Some(b'0') => {
                self.next();
                Ok(false)
            }
            Some(b'1') => {
                self.next();
                Ok(true)
            }
            _ => self.error("expected boolean ('0' or '1')"),
        }
    }

    pub(crate) fn parse_string(&mut self) -> SFVResult<String> {
        // https://httpwg.org/specs/rfc8941.html#parse-string

        if self.peek() != Some(b'"') {
            return self.error(r#"expected start of string ('"')"#);
        }

        self.next();

        let mut output_string = StdString::new();
        while let Some(curr_char) = self.peek() {
            match curr_char {
                b'"' => {
                    self.next();
                    return Ok(String::from_string(output_string).unwrap());
                }
                0x00..=0x1f | 0x7f..=0xff => {
                    return self.error("invalid string character");
                }
                b'\\' => {
                    self.next();
                    match self.peek() {
                        Some(c @ b'\\' | c @ b'"') => {
                            self.next();
                            output_string.push(c as char);
                        }
                        None => return self.error("unterminated escape sequence"),
                        Some(_) => return self.error("invalid escape sequence"),
                    }
                }
                _ => {
                    self.next();
                    output_string.push(curr_char as char);
                }
            }
        }
        self.error("unterminated string")
    }

    pub(crate) fn parse_token(&mut self) -> SFVResult<Token> {
        // https://httpwg.org/specs/rfc8941.html#parse-token

        let mut output_string = StdString::new();

        match self.peek() {
            Some(c) if utils::is_allowed_start_token_char(c) => {
                self.next();
                output_string.push(c as char);
            }
            _ => return self.error("expected start of token"),
        }

        loop {
            match self.peek() {
                Some(c) if utils::is_allowed_inner_token_char(c) => {
                    self.next();
                    output_string.push(c as char);
                }
                _ => return Ok(Token::from_string(output_string).unwrap()),
            }
        }
    }

    pub(crate) fn parse_byte_sequence(&mut self) -> SFVResult<Vec<u8>> {
        // https://httpwg.org/specs/rfc8941.html#parse-binary

        if self.peek() != Some(b':') {
            return self.error("expected start of byte sequence (':')");
        }

        self.next();
        let start = self.index;

        loop {
            match self.next() {
                Some(b':') => break,
                Some(_) => {}
                None => return self.error("unterminated byte sequence"),
            }
        }

        let colon_index = self.index - 1;

        match base64::Engine::decode(&utils::BASE64, &self.input[start..colon_index]) {
            Ok(content) => Ok(content),
            Err(err) => {
                let index = match err {
                    base64::DecodeError::InvalidByte(offset, _)
                    | base64::DecodeError::InvalidLastSymbol(offset, _) => start + offset,
                    // Report these two at the position of the last base64
                    // character, since they correspond to errors in the input
                    // as a whole.
                    base64::DecodeError::InvalidLength(_) | base64::DecodeError::InvalidPadding => {
                        colon_index - 1
                    }
                };

                Err(Error::with_index("invalid byte sequence", index))
            }
        }
    }

    pub(crate) fn parse_number(&mut self) -> SFVResult<Num> {
        // https://httpwg.org/specs/rfc8941.html#parse-number

        fn char_to_i64(c: u8) -> i64 {
            (c - b'0') as i64
        }

        let sign = if let Some(b'-') = self.peek() {
            self.next();
            -1
        } else {
            1
        };

        let mut magnitude = match self.peek() {
            Some(c @ b'0'..=b'9') => {
                self.next();
                char_to_i64(c)
            }
            _ => return self.error("expected digit"),
        };

        let mut digits = 1;

        loop {
            match self.peek() {
                Some(b'.') => {
                    if digits > 12 {
                        return self.error("too many digits before decimal point");
                    }
                    self.next();
                    break;
                }
                Some(c @ b'0'..=b'9') => {
                    digits += 1;
                    if digits > 15 {
                        return self.error("too many digits");
                    }
                    self.next();
                    magnitude = magnitude * 10 + char_to_i64(c);
                }
                _ => return Ok(Num::Integer(Integer::try_from(sign * magnitude).unwrap())),
            }
        }

        digits = 0;

        while let Some(c @ b'0'..=b'9') = self.peek() {
            if digits == 3 {
                return self.error("too many digits after decimal point");
            }

            self.next();
            magnitude = magnitude * 10 + char_to_i64(c);
            digits += 1;
        }

        if digits == 0 {
            // Report the error at the position of the decimal itself, rather
            // than the next position.
            Err(Error::with_index("trailing decimal point", self.index - 1))
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

        while let Some(b';') = self.peek() {
            self.next();
            self.consume_sp_chars();

            let param_name = self.parse_key()?;
            let param_value = match self.peek() {
                Some(b'=') => {
                    self.next();
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

    pub(crate) fn parse_key(&mut self) -> SFVResult<Key> {
        let mut output = StdString::new();

        match self.peek() {
            Some(c) if utils::is_allowed_start_key_char(c) => {
                self.next();
                output.push(c as char);
            }
            _ => return self.error("expected start of key ('a'-'z' or '*')"),
        }

        loop {
            match self.peek() {
                Some(c) if utils::is_allowed_inner_key_char(c) => {
                    self.next();
                    output.push(c as char);
                }
                _ => return Ok(Key::from_string(output).unwrap()),
            }
        }
    }

    fn consume_ows_chars(&mut self) {
        while let Some(b' ' | b'\t') = self.peek() {
            self.next();
        }
    }

    fn consume_sp_chars(&mut self) {
        while let Some(b' ') = self.peek() {
            self.next();
        }
    }

    #[cfg(test)]
    pub(crate) fn remaining(&self) -> &[u8] {
        &self.input[self.index..]
    }
}
