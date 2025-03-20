use crate::utils;
use crate::visitor::*;
use crate::{
    BareItemFromInput, Decimal, Error, Integer, KeyRef, Num, SFVResult, String, StringRef, TokenRef,
};

#[cfg(feature = "parsed-types")]
use crate::{Dictionary, Item, List};

use std::borrow::Cow;
use std::convert::TryFrom;
use std::string::String as StdString;

fn parse_item<'a>(parser: &mut Parser<'a>, visitor: impl ItemVisitor<'a>) -> SFVResult<()> {
    // https://httpwg.org/specs/rfc8941.html#parse-item
    let param_visitor = visitor
        .bare_item(parser.parse_bare_item()?)
        .map_err(Error::custom)?;
    parser.parse_parameters(param_visitor)
}

fn parse_list<'a>(
    parser: &mut Parser<'a>,
    visitor: &mut (impl ?Sized + ListVisitor<'a>),
) -> SFVResult<()> {
    // https://httpwg.org/specs/rfc8941.html#parse-list
    // List represents an array of (item_or_inner_list, parameters)

    while parser.peek().is_some() {
        parser.parse_list_entry(visitor.entry().map_err(Error::custom)?)?;

        parser.consume_ows_chars();

        if parser.peek().is_none() {
            return Ok(());
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

    Ok(())
}

fn parse_dictionary<'a>(
    parser: &mut Parser<'a>,
    visitor: &mut (impl ?Sized + DictionaryVisitor<'a>),
) -> SFVResult<()> {
    while parser.peek().is_some() {
        // Note: It is up to the visitor to properly handle duplicate keys.
        let entry_visitor = visitor.entry(parser.parse_key()?).map_err(Error::custom)?;

        if let Some(b'=') = parser.peek() {
            parser.next();
            parser.parse_list_entry(entry_visitor)?;
        } else {
            let param_visitor = entry_visitor
                .bare_item(BareItemFromInput::from(true))
                .map_err(Error::custom)?;
            parser.parse_parameters(param_visitor)?;
        }

        parser.consume_ows_chars();

        if parser.peek().is_none() {
            return Ok(());
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
    Ok(())
}

/// Exposes methods for parsing input into a structured field value.
pub struct Parser<'a> {
    input: &'a [u8],
    index: usize,
}

impl<'a> Parser<'a> {
    /// Creates a parser from the given input.
    pub fn from_bytes(input: &'a [u8]) -> Self {
        Self { input, index: 0 }
    }

    /// Creates a parser from the given input.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(input: &'a str) -> Self {
        Self::from_bytes(input.as_bytes())
    }

    /// Parses input into a structured field value of `Dictionary` type.
    #[cfg(feature = "parsed-types")]
    pub fn parse_dictionary(self) -> SFVResult<Dictionary> {
        let mut dict = Dictionary::new();
        self.parse_dictionary_with_visitor(&mut dict)?;
        Ok(dict)
    }

    /// Parses input into a structured field value of `Dictionary` type, using
    /// the given visitor.
    #[cfg_attr(
        feature = "parsed-types",
        doc = r##"
This can also be used to parse a dictionary that is split into multiple lines by merging
them into an existing structure:

```
# use sfv::{Parser, SerializeValue};
# fn main() -> Result<(), sfv::Error> {
let mut dict = Parser::from_str("a=1").parse_dictionary()?;

Parser::from_str("b=2").parse_dictionary_with_visitor(&mut dict)?;

assert_eq!(
    dict.serialize_value()?,
    "a=1, b=2",
);
# Ok(())
# }
"##
    )]
    pub fn parse_dictionary_with_visitor(
        self,
        visitor: &mut (impl ?Sized + DictionaryVisitor<'a>),
    ) -> SFVResult<()> {
        self.parse(|parser| parse_dictionary(parser, visitor))
    }

    /// Parses input into a structured field value of `List` type.
    #[cfg(feature = "parsed-types")]
    pub fn parse_list(self) -> SFVResult<List> {
        let mut list = List::new();
        self.parse_list_with_visitor(&mut list)?;
        Ok(list)
    }

    /// Parses input into a structured field value of `List` type, using the
    /// given visitor.
    #[cfg_attr(
        feature = "parsed-types",
        doc = r##"
This can also be used to parse a list that is split into multiple lines by merging them
into an existing structure:
```
# use sfv::{Parser, SerializeValue};
# fn main() -> Result<(), sfv::Error> {
let mut list = Parser::from_str("11, (12 13)").parse_list()?;

Parser::from_str(r#""foo",        "bar""#).parse_list_with_visitor(&mut list)?;

assert_eq!(
    list.serialize_value()?,
    r#"11, (12 13), "foo", "bar""#,
);
# Ok(())
# }
```
"##
    )]
    pub fn parse_list_with_visitor(
        self,
        visitor: &mut (impl ?Sized + ListVisitor<'a>),
    ) -> SFVResult<()> {
        self.parse(|parser| parse_list(parser, visitor))
    }

    /// Parses input into a structured field value of `Item` type.
    #[cfg(feature = "parsed-types")]
    pub fn parse_item(self) -> SFVResult<Item> {
        let mut item = Item::new(false);
        self.parse_item_with_visitor(&mut item)?;
        Ok(item)
    }

    /// Parses input into a structured field value of `Item` type, using the
    /// given visitor.
    pub fn parse_item_with_visitor(self, visitor: impl ItemVisitor<'a>) -> SFVResult<()> {
        self.parse(|parser| parse_item(parser, visitor))
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
    fn parse(mut self, f: impl FnOnce(&mut Self) -> SFVResult<()>) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#text-parse

        self.consume_sp_chars();

        f(&mut self)?;

        self.consume_sp_chars();

        if self.peek().is_some() {
            self.error("trailing characters after parsed value")
        } else {
            Ok(())
        }
    }

    fn parse_list_entry(&mut self, visitor: impl EntryVisitor<'a>) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#parse-item-or-list
        // ListEntry represents a tuple (item_or_inner_list, parameters)

        match self.peek() {
            Some(b'(') => self.parse_inner_list(visitor.inner_list().map_err(Error::custom)?),
            _ => parse_item(self, visitor),
        }
    }

    pub(crate) fn parse_inner_list(
        &mut self,
        mut visitor: impl InnerListVisitor<'a>,
    ) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#parse-innerlist

        if Some(b'(') != self.peek() {
            return self.error("expected start of inner list");
        }

        self.next();

        while self.peek().is_some() {
            self.consume_sp_chars();

            if Some(b')') == self.peek() {
                self.next();
                let param_visitor = visitor.finish().map_err(Error::custom)?;
                return self.parse_parameters(param_visitor);
            }

            parse_item(self, visitor.item().map_err(Error::custom)?)?;

            if let Some(c) = self.peek() {
                if c != b' ' && c != b')' {
                    return self.error("expected inner list delimiter (' ' or ')')");
                }
            }
        }

        self.error("unterminated inner list")
    }

    pub(crate) fn parse_bare_item(&mut self) -> SFVResult<BareItemFromInput<'a>> {
        // https://httpwg.org/specs/rfc8941.html#parse-bare-item

        match self.peek() {
            Some(b'?') => Ok(BareItemFromInput::Boolean(self.parse_bool()?)),
            Some(b'"') => Ok(BareItemFromInput::String(self.parse_string()?)),
            Some(b':') => Ok(BareItemFromInput::ByteSeq(self.parse_byte_sequence()?)),
            Some(c) if utils::is_allowed_start_token_char(c) => {
                Ok(BareItemFromInput::Token(self.parse_token()?))
            }
            Some(c) if c == b'-' || c.is_ascii_digit() => match self.parse_number()? {
                Num::Decimal(val) => Ok(BareItemFromInput::Decimal(val)),
                Num::Integer(val) => Ok(BareItemFromInput::Integer(val)),
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

    pub(crate) fn parse_string(&mut self) -> SFVResult<Cow<'a, StringRef>> {
        // https://httpwg.org/specs/rfc8941.html#parse-string

        if self.peek() != Some(b'"') {
            return self.error(r#"expected start of string ('"')"#);
        }

        self.next();

        let start = self.index;
        let mut output = Cow::Borrowed(&[] as &[u8]);

        while let Some(curr_char) = self.peek() {
            match curr_char {
                b'"' => {
                    self.next();
                    // TODO: The UTF-8 validation is redundant with the preceding character checks, but
                    // its removal is only possible with unsafe code.
                    return Ok(match output {
                        Cow::Borrowed(output) => {
                            let output = std::str::from_utf8(output).unwrap();
                            Cow::Borrowed(StringRef::from_str(output).unwrap())
                        }
                        Cow::Owned(output) => {
                            let output = StdString::from_utf8(output).unwrap();
                            Cow::Owned(String::from_string(output).unwrap())
                        }
                    });
                }
                0x00..=0x1f | 0x7f..=0xff => {
                    return self.error("invalid string character");
                }
                b'\\' => {
                    self.next();
                    match self.peek() {
                        Some(c @ b'\\' | c @ b'"') => {
                            self.next();
                            output.to_mut().push(c);
                        }
                        None => return self.error("unterminated escape sequence"),
                        Some(_) => return self.error("invalid escape sequence"),
                    }
                }
                _ => {
                    self.next();
                    match output {
                        Cow::Borrowed(ref mut output) => *output = &self.input[start..self.index],
                        Cow::Owned(ref mut output) => output.push(curr_char),
                    }
                }
            }
        }
        self.error("unterminated string")
    }

    fn parse_non_empty_str(
        &mut self,
        is_allowed_start_char: impl FnOnce(u8) -> bool,
        is_allowed_inner_char: impl Fn(u8) -> bool,
    ) -> Option<&'a str> {
        let start = self.index;

        match self.peek() {
            Some(c) if is_allowed_start_char(c) => {
                self.next();
            }
            _ => return None,
        }

        loop {
            match self.peek() {
                Some(c) if is_allowed_inner_char(c) => {
                    self.next();
                }
                // TODO: The UTF-8 validation is redundant with the preceding character checks, but
                // its removal is only possible with unsafe code.
                _ => return Some(std::str::from_utf8(&self.input[start..self.index]).unwrap()),
            }
        }
    }

    pub(crate) fn parse_token(&mut self) -> SFVResult<&'a TokenRef> {
        // https://httpwg.org/specs/rfc8941.html#parse-token

        match self.parse_non_empty_str(
            utils::is_allowed_start_token_char,
            utils::is_allowed_inner_token_char,
        ) {
            None => self.error("expected start of token"),
            Some(str) => Ok(TokenRef::from_validated_str(str)),
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

        magnitude *= 1000;
        let mut scale = 100;

        while let Some(c @ b'0'..=b'9') = self.peek() {
            if scale == 0 {
                return self.error("too many digits after decimal point");
            }

            self.next();
            magnitude += char_to_i64(c) * scale;
            scale /= 10;
        }

        if scale == 100 {
            // Report the error at the position of the decimal itself, rather
            // than the next position.
            Err(Error::with_index("trailing decimal point", self.index - 1))
        } else {
            Ok(Num::Decimal(Decimal::from_integer_scaled_1000(
                Integer::try_from(sign * magnitude).unwrap(),
            )))
        }
    }

    pub(crate) fn parse_parameters(
        &mut self,
        mut visitor: impl ParameterVisitor<'a>,
    ) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc8941.html#parse-param

        while let Some(b';') = self.peek() {
            self.next();
            self.consume_sp_chars();

            let param_name = self.parse_key()?;
            let param_value = match self.peek() {
                Some(b'=') => {
                    self.next();
                    self.parse_bare_item()?
                }
                _ => BareItemFromInput::Boolean(true),
            };
            // Note: It is up to the visitor to properly handle duplicate keys.
            visitor
                .parameter(param_name, param_value)
                .map_err(Error::custom)?;
        }

        visitor.finish().map_err(Error::custom)
    }

    pub(crate) fn parse_key(&mut self) -> SFVResult<&'a KeyRef> {
        // https://httpwg.org/specs/rfc8941.html#parse-key

        match self.parse_non_empty_str(
            utils::is_allowed_start_key_char,
            utils::is_allowed_inner_key_char,
        ) {
            None => self.error("expected start of key ('a'-'z' or '*')"),
            Some(str) => Ok(KeyRef::from_validated_str(str)),
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
