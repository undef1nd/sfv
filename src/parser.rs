use std::{borrow::Cow, string::String as StdString};

use crate::{
    utils,
    visitor::{
        DictionaryVisitor, EntryVisitor, InnerListVisitor, ItemVisitor, ListVisitor,
        ParameterVisitor,
    },
    BareItemFromInput, Date, Decimal, Error, Integer, KeyRef, Num, SFVResult, String, StringRef,
    TokenRef, Version,
};
#[cfg(feature = "parsed-types")]
use crate::{Dictionary, Item, List};

fn parse_item<'de>(parser: &mut Parser<'de>, visitor: impl ItemVisitor<'de>) -> SFVResult<()> {
    // https://httpwg.org/specs/rfc9651.html#parse-item
    let param_visitor = visitor
        .bare_item(parser.parse_bare_item()?)
        .map_err(Error::custom)?;
    parser.parse_parameters(param_visitor)
}

fn parse_comma_separated<'de>(
    parser: &mut Parser<'de>,
    mut parse_member: impl FnMut(&mut Parser<'de>) -> SFVResult<()>,
) -> SFVResult<()> {
    while parser.peek().is_some() {
        parse_member(parser)?;

        parser.consume_ows_chars();

        if parser.peek().is_none() {
            return Ok(());
        }

        let comma_index = parser.index;

        if let Some(c) = parser.peek() {
            if c != b',' {
                return parser.error("trailing characters after member");
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
pub struct Parser<'de> {
    input: &'de [u8],
    index: usize,
    version: Version,
}

impl<'de> Parser<'de> {
    /// Creates a parser from the given input with [`Version::Rfc9651`].
    pub fn new(input: &'de (impl ?Sized + AsRef<[u8]>)) -> Self {
        Self {
            input: input.as_ref(),
            index: 0,
            version: Version::Rfc9651,
        }
    }

    /// Sets the parser's version and returns it.
    #[must_use]
    pub fn with_version(mut self, version: Version) -> Self {
        self.version = version;
        self
    }

    /// Parses input into a structured field value of `Dictionary` type.
    ///
    /// # Errors
    /// When the parsing process is unsuccessful.
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
        doc = r#"

This can also be used to parse a dictionary that is split into multiple lines by merging
them into an existing structure:

```
# use sfv::{Parser, SerializeValue};
# fn main() -> Result<(), sfv::Error> {
let mut dict = Parser::new("a=1").parse_dictionary()?;

Parser::new("b=2").parse_dictionary_with_visitor(&mut dict)?;

assert_eq!(
    dict.serialize_value().as_deref(),
    Some("a=1, b=2"),
);
# Ok(())
# }
```
"#
    )]
    ///
    /// # Errors
    /// When the parsing process is unsuccessful, including any error raised by a visitor.
    pub fn parse_dictionary_with_visitor(
        self,
        visitor: &mut (impl ?Sized + DictionaryVisitor<'de>),
    ) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc9651.html#parse-dictionary
        self.parse(move |parser| {
            parse_comma_separated(parser, |parser| {
                // Note: It is up to the visitor to properly handle duplicate keys.
                let entry_visitor = visitor.entry(parser.parse_key()?).map_err(Error::custom)?;

                if let Some(b'=') = parser.peek() {
                    parser.next();
                    parser.parse_list_entry(entry_visitor)
                } else {
                    let param_visitor = entry_visitor
                        .bare_item(BareItemFromInput::from(true))
                        .map_err(Error::custom)?;
                    parser.parse_parameters(param_visitor)
                }
            })
        })
    }

    /// Parses input into a structured field value of `List` type.
    ///
    /// # Errors
    /// When the parsing process is unsuccessful.
    #[cfg(feature = "parsed-types")]
    pub fn parse_list(self) -> SFVResult<List> {
        let mut list = List::new();
        self.parse_list_with_visitor(&mut list)?;
        Ok(list)
    }

    /// Parses input into a structured field value of `List` type, using the
    /// given visitor.
    #[allow(clippy::needless_raw_string_hashes)] // false positive: https://github.com/rust-lang/rust-clippy/issues/11737
    #[cfg_attr(
        feature = "parsed-types",
        doc = r##"

This can also be used to parse a list that is split into multiple lines by merging them
into an existing structure:
```
# use sfv::{Parser, SerializeValue};
# fn main() -> Result<(), sfv::Error> {
let mut list = Parser::new("11, (12 13)").parse_list()?;

Parser::new(r#""foo",        "bar""#).parse_list_with_visitor(&mut list)?;

assert_eq!(
    list.serialize_value().as_deref(),
    Some(r#"11, (12 13), "foo", "bar""#),
);
# Ok(())
# }
```
"##
    )]
    ///
    /// # Errors
    /// When the parsing process is unsuccessful, including any error raised by a visitor.
    pub fn parse_list_with_visitor(
        self,
        visitor: &mut (impl ?Sized + ListVisitor<'de>),
    ) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc9651.html#parse-list
        self.parse(|parser| {
            parse_comma_separated(parser, |parser| {
                parser.parse_list_entry(visitor.entry().map_err(Error::custom)?)
            })
        })
    }

    /// Parses input into a structured field value of `Item` type.
    ///
    /// # Errors
    /// When the parsing process is unsuccessful.
    #[cfg(feature = "parsed-types")]
    pub fn parse_item(self) -> SFVResult<Item> {
        let mut item = Item::new(false);
        self.parse_item_with_visitor(&mut item)?;
        Ok(item)
    }

    /// Parses input into a structured field value of `Item` type, using the
    /// given visitor.
    ///
    /// # Errors
    /// When the parsing process is unsuccessful, including any error raised by a visitor.
    pub fn parse_item_with_visitor(self, visitor: impl ItemVisitor<'de>) -> SFVResult<()> {
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
        // https://httpwg.org/specs/rfc9651.html#text-parse

        self.consume_sp_chars();

        f(&mut self)?;

        self.consume_sp_chars();

        if self.peek().is_some() {
            self.error("trailing characters after parsed value")
        } else {
            Ok(())
        }
    }

    fn parse_list_entry(&mut self, visitor: impl EntryVisitor<'de>) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc9651.html#parse-item-or-list
        // ListEntry represents a tuple (item_or_inner_list, parameters)

        match self.peek() {
            Some(b'(') => self.parse_inner_list(visitor.inner_list().map_err(Error::custom)?),
            _ => parse_item(self, visitor),
        }
    }

    pub(crate) fn parse_inner_list(
        &mut self,
        mut visitor: impl InnerListVisitor<'de>,
    ) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc9651.html#parse-innerlist

        debug_assert_eq!(self.peek(), Some(b'('), "expected start of inner list");
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

    pub(crate) fn parse_bare_item(&mut self) -> SFVResult<BareItemFromInput<'de>> {
        // https://httpwg.org/specs/rfc9651.html#parse-bare-item

        match self.peek() {
            Some(b'?') => Ok(BareItemFromInput::Boolean(self.parse_bool()?)),
            Some(b'"') => Ok(BareItemFromInput::String(self.parse_string()?)),
            Some(b':') => Ok(BareItemFromInput::ByteSequence(self.parse_byte_sequence()?)),
            Some(b'@') => Ok(BareItemFromInput::Date(self.parse_date()?)),
            Some(b'%') => Ok(BareItemFromInput::DisplayString(
                self.parse_display_string()?,
            )),
            Some(c) if utils::is_allowed_start_token_char(c) => {
                Ok(BareItemFromInput::Token(self.parse_token()))
            }
            Some(c) if c == b'-' || c.is_ascii_digit() => match self.parse_number()? {
                Num::Decimal(val) => Ok(BareItemFromInput::Decimal(val)),
                Num::Integer(val) => Ok(BareItemFromInput::Integer(val)),
            },
            _ => self.error("expected start of bare item"),
        }
    }

    pub(crate) fn parse_bool(&mut self) -> SFVResult<bool> {
        // https://httpwg.org/specs/rfc9651.html#parse-boolean

        debug_assert_eq!(self.peek(), Some(b'?'), "expected start of boolean");
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

    pub(crate) fn parse_string(&mut self) -> SFVResult<Cow<'de, StringRef>> {
        // https://httpwg.org/specs/rfc9651.html#parse-string

        debug_assert_eq!(self.peek(), Some(b'"'), "expected start of string");
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
                        Some(c @ (b'\\' | b'"')) => {
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

    fn parse_non_empty_str(&mut self, is_allowed_inner_char: impl Fn(u8) -> bool) -> &'de str {
        debug_assert!(self.peek().is_some());
        let start = self.index;
        self.next();

        loop {
            match self.peek() {
                Some(c) if is_allowed_inner_char(c) => {
                    self.next();
                }
                // TODO: The UTF-8 validation is redundant with the preceding character checks, but
                // its removal is only possible with unsafe code.
                _ => return std::str::from_utf8(&self.input[start..self.index]).unwrap(),
            }
        }
    }

    pub(crate) fn parse_token(&mut self) -> &'de TokenRef {
        // https://httpwg.org/specs/9651.html#parse-token

        debug_assert!(
            self.peek().is_some_and(utils::is_allowed_start_token_char),
            "expected start of token"
        );

        TokenRef::from_validated_str(self.parse_non_empty_str(utils::is_allowed_inner_token_char))
    }

    pub(crate) fn parse_byte_sequence(&mut self) -> SFVResult<Vec<u8>> {
        // https://httpwg.org/specs/rfc9651.html#parse-binary

        debug_assert_eq!(self.peek(), Some(b':'), "expected start of byte sequence");
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
        // https://httpwg.org/specs/rfc9651.html#parse-number

        fn char_to_i64(c: u8) -> i64 {
            i64::from(c - b'0')
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

    pub(crate) fn parse_date(&mut self) -> SFVResult<Date> {
        // https://httpwg.org/specs/rfc9651.html#parse-date

        debug_assert_eq!(self.peek(), Some(b'@'), "expected start of date");

        match self.version {
            Version::Rfc8941 => return self.error("RFC 8941 does not support dates"),
            Version::Rfc9651 => {}
        }

        let start = self.index;
        self.next();

        match self.parse_number()? {
            Num::Integer(seconds) => Ok(Date::from_unix_seconds(seconds)),
            Num::Decimal(_) => Err(Error::with_index(
                "date must be an integer number of seconds",
                start,
            )),
        }
    }

    pub(crate) fn parse_display_string(&mut self) -> SFVResult<Cow<'de, str>> {
        // https://httpwg.org/specs/rfc9651.html#parse-display

        debug_assert_eq!(self.peek(), Some(b'%'), "expected start of display string");

        match self.version {
            Version::Rfc8941 => return self.error("RFC 8941 does not support display strings"),
            Version::Rfc9651 => {}
        }

        self.next();

        if self.peek() != Some(b'"') {
            return self.error(r#"expected '"'"#);
        }

        self.next();

        let start = self.index;
        let mut output = Cow::Borrowed(&[] as &[u8]);

        while let Some(curr_char) = self.peek() {
            match curr_char {
                b'"' => {
                    self.next();
                    return match output {
                        Cow::Borrowed(output) => match std::str::from_utf8(output) {
                            Ok(output) => Ok(Cow::Borrowed(output)),
                            Err(err) => Err(Error::with_index(
                                "invalid UTF-8 in display string",
                                start + err.valid_up_to(),
                            )),
                        },
                        Cow::Owned(output) => match StdString::from_utf8(output) {
                            Ok(output) => Ok(Cow::Owned(output)),
                            Err(err) => Err(Error::with_index(
                                "invalid UTF-8 in display string",
                                start + err.utf8_error().valid_up_to(),
                            )),
                        },
                    };
                }
                0x00..=0x1f | 0x7f..=0xff => {
                    return self.error("invalid display string character");
                }
                b'%' => {
                    self.next();

                    let mut octet = 0;

                    for _ in 0..2 {
                        octet = (octet << 4)
                            + match self.peek() {
                                Some(c @ b'0'..=b'9') => {
                                    self.next();
                                    c - b'0'
                                }
                                Some(c @ b'a'..=b'f') => {
                                    self.next();
                                    c - b'a' + 10
                                }
                                None => return self.error("unterminated escape sequence"),
                                Some(_) => return self.error("invalid escape sequence"),
                            };
                    }

                    output.to_mut().push(octet);
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
        self.error("unterminated display string")
    }

    pub(crate) fn parse_parameters(
        &mut self,
        mut visitor: impl ParameterVisitor<'de>,
    ) -> SFVResult<()> {
        // https://httpwg.org/specs/rfc9651.html#parse-param

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

    pub(crate) fn parse_key(&mut self) -> SFVResult<&'de KeyRef> {
        // https://httpwg.org/specs/rfc9651.html#parse-key

        if !self.peek().is_some_and(utils::is_allowed_start_key_char) {
            return self.error("expected start of key ('a'-'z' or '*')");
        }

        Ok(KeyRef::from_validated_str(
            self.parse_non_empty_str(utils::is_allowed_inner_key_char),
        ))
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
