use crate::utils;
use indexmap::IndexMap;
use rust_decimal::prelude::*;
use std::fmt::Debug;
use std::iter::Peekable;
use std::str::{from_utf8, Chars};

type ParseResult<T> = Result<T, &'static str>;

pub type Dictionary = IndexMap<String, ListEntry>;
pub type Parameters = IndexMap<String, BareItem>;
pub type List = Vec<ListEntry>;

#[derive(Debug, PartialEq)]
pub enum ListEntry {
    Item(Item),
    InnerList(InnerList),
}

impl From<Item> for ListEntry {
    fn from(item: Item) -> Self {
        ListEntry::Item(item)
    }
}

impl From<InnerList> for ListEntry {
    fn from(item: InnerList) -> Self {
        ListEntry::InnerList(item)
    }
}

#[derive(Debug, PartialEq)]
pub struct InnerList(pub Vec<Item>, pub Parameters);

#[derive(Debug, PartialEq)]
pub struct Item(pub BareItem, pub Parameters);

#[derive(Debug, PartialEq)]
pub enum Num {
    Decimal(Decimal),
    Integer(i64),
}

#[derive(Debug, PartialEq)]
pub enum BareItem {
    Number(Num),
    String(String),
    ByteSeq(Vec<u8>),
    Boolean(bool),
    Token(String),
}

impl From<i64> for BareItem {
    fn from(item: i64) -> Self {
        BareItem::Number(Num::Integer(item))
    }
}

impl From<Decimal> for BareItem {
    fn from(item: Decimal) -> Self {
        BareItem::Number(Num::Decimal(item))
    }
}

#[derive(Debug, PartialEq)]
pub enum Header {
    List(List),
    Dictionary(Dictionary),
    Item(Item),
}

#[derive(Debug)]
pub struct Parser;

impl Parser {
    pub fn parse(input_bytes: &[u8], header_type: &str) -> ParseResult<Header> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#text-parse
        if !input_bytes.is_ascii() {
            return Err("parse: non-ASCII characters in input");
        }

        let mut input_chars = from_utf8(input_bytes)
            .map_err(|_| "parse: conversion from bytes to str failed")?
            .chars()
            .peekable();
        utils::consume_sp_chars(&mut input_chars);

        let output = match header_type {
            "list" => Header::List(Self::parse_list(&mut input_chars)?),
            "dictionary" => Header::Dictionary(Self::parse_dict(&mut input_chars)?),
            "item" => Header::Item(Self::parse_item(&mut input_chars)?),
            _ => return Err("parse: unrecognized header type"),
        };

        utils::consume_sp_chars(&mut input_chars);

        if input_chars.next().is_some() {
            return Err("parse: trailing text after parsed value");
        };
        Ok(output)
    }

    fn parse_dict(input_chars: &mut Peekable<Chars>) -> ParseResult<Dictionary> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-dictionary

        let mut dict = Dictionary::new();

        while input_chars.peek().is_some() {
            let this_key = Self::parse_key(input_chars)?;

            if let Some('=') = input_chars.peek() {
                input_chars.next();
                let member = Self::parse_list_entry(input_chars)?;
                dict.insert(this_key, member);
            } else {
                let value = true;
                let params = Self::parse_parameters(input_chars)?;
                let member = Item(BareItem::Boolean(value), params);
                dict.insert(this_key, member.into());
            }

            utils::consume_ows_chars(input_chars);

            if input_chars.peek().is_none() {
                return Ok(dict);
            }

            if let Some(c) = input_chars.next() {
                if c != ',' {
                    return Err("parse_dict: trailing text after member in dict");
                }
            }

            utils::consume_ows_chars(input_chars);

            if input_chars.peek().is_none() {
                return Err("parse_dict: trailing comma at the end of the list");
            }
        }
        Ok(dict)
    }

    fn parse_list(input_chars: &mut Peekable<Chars>) -> ParseResult<List> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-list
        // List represents an array of (item_or_inner_list, parameters)

        let mut members = vec![];

        while input_chars.peek().is_some() {
            members.push(Self::parse_list_entry(input_chars)?);

            utils::consume_ows_chars(input_chars);

            if input_chars.peek().is_none() {
                return Ok(members);
            }

            if let Some(c) = input_chars.next() {
                if c != ',' {
                    return Err("parse_list: trailing text after item in list");
                }
            }

            utils::consume_ows_chars(input_chars);

            if input_chars.peek().is_none() {
                return Err("parse_list: trailing comma at the end of the list");
            }
        }

        Ok(members)
    }

    fn parse_list_entry(input_chars: &mut Peekable<Chars>) -> ParseResult<ListEntry> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-item-or-list
        // ListEntry represents a tuple (item_or_inner_list, parameters)

        match input_chars.peek() {
            Some('(') => {
                let parsed = Self::parse_inner_list(input_chars)?;
                Ok(ListEntry::InnerList(parsed))
            }
            _ => {
                let parsed = Self::parse_item(input_chars)?;
                Ok(ListEntry::Item(parsed))
            }
        }
    }

    fn parse_inner_list(input_chars: &mut Peekable<Chars>) -> ParseResult<InnerList> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-innerlist

        if Some('(') != input_chars.next() {
            return Err("parse_inner_list: input does not start with '('");
        }

        let mut inner_list = Vec::new();
        while input_chars.peek().is_some() {
            utils::consume_sp_chars(input_chars);

            if Some(&')') == input_chars.peek() {
                input_chars.next();
                let params = Self::parse_parameters(input_chars)?;
                return Ok(InnerList(inner_list, params));
            }

            let parsed_item = Self::parse_item(input_chars)?;
            inner_list.push(parsed_item);

            if let Some(c) = input_chars.peek() {
                if c != &' ' && c != &')' {
                    return Err("parse_inner_list: bad delimitation");
                }
            }
        }

        Err("parse_inner_list: the end of the inner list was not found")
    }

    fn parse_item(input_chars: &mut Peekable<Chars>) -> ParseResult<Item> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-item
        let bare_item = Self::parse_bare_item(input_chars)?;
        let parameters = Self::parse_parameters(input_chars)?;

        Ok(Item(bare_item, parameters))
    }

    fn parse_bare_item(mut input_chars: &mut Peekable<Chars>) -> ParseResult<BareItem> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-bare-item
        if input_chars.peek().is_none() {
            return Err("parse_bare_item: empty item");
        }

        match input_chars.peek() {
            Some(&'?') => Ok(BareItem::Boolean(Self::parse_bool(&mut input_chars)?)),
            Some(&'"') => Ok(BareItem::String(Self::parse_string(&mut input_chars)?)),
            Some(&':') => Ok(BareItem::ByteSeq(Self::parse_byte_sequence(
                &mut input_chars,
            )?)),
            Some(&c) if c == '*' || c.is_ascii_alphabetic() => {
                Ok(BareItem::Token(Self::parse_token(&mut input_chars)?))
            }
            Some(&c) if c == '-' || c.is_ascii_digit() => {
                Ok(BareItem::Number(Self::parse_number(&mut input_chars)?))
            }
            _ => Err("parse_bare_item: item type can't be identified"),
        }
    }

    fn parse_bool(input_chars: &mut Peekable<Chars>) -> ParseResult<bool> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-boolean

        if input_chars.next() != Some('?') {
            return Err("parse_bool: first char is not '?'");
        }

        match input_chars.next() {
            Some('0') => Ok(false),
            Some('1') => Ok(true),
            _ => Err("parse_bool: invalid variant"),
        }
    }

    fn parse_string(input_chars: &mut Peekable<Chars>) -> ParseResult<String> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-string

        if input_chars.next() != Some('\"') {
            return Err("parse_string: first char is not '\"'");
        }

        let mut output_string = String::from("");
        while let Some(curr_char) = input_chars.next() {
            match curr_char {
                '\"' => return Ok(output_string),
                '\x7f' | '\x00'..='\x1f' => return Err("parse_string: not a visible char"),
                '\\' => match input_chars.next() {
                    Some(c) if c == '\\' || c == '\"' => {
                        output_string.push(c);
                    }
                    None => return Err("parse_string: last input char is '\\'"),
                    _ => return Err("parse_string: disallowed char after '\\'"),
                },
                _ => output_string.push(curr_char),
            }
        }
        Err("parse_string: no closing '\"'")
    }

    fn parse_token(input_chars: &mut Peekable<Chars>) -> ParseResult<String> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-token

        if let Some(first_char) = input_chars.peek() {
            if !first_char.is_ascii_alphabetic() && first_char != &'*' {
                return Err("parse_token: first char is not ALPHA or '*'");
            }
        } else {
            return Err("parse_token: empty input string");
        }

        let mut output_string = String::from("");
        while let Some(curr_char) = input_chars.peek() {
            if !utils::is_tchar(*curr_char) && curr_char != &':' && curr_char != &'/' {
                return Ok(output_string);
            }

            match input_chars.next() {
                Some(c) => output_string.push(c),
                None => return Err("parse_token: end of the string"),
            }
        }
        Ok(output_string)
    }

    fn parse_byte_sequence(input_chars: &mut Peekable<Chars>) -> ParseResult<Vec<u8>> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-binary

        if input_chars.next() != Some(':') {
            return Err("parse_byte_seq: first char is not ':'");
        }

        if !input_chars.clone().any(|c| c == ':') {
            return Err("parse_byte_seq: no closing ':'");
        }

        let b64_content = input_chars.take_while(|c| c != &':').collect::<String>();
        if !b64_content
            .chars()
            .all(|c| utils::is_allowed_b64_content(c))
        {
            return Err("parse_byte_seq: invalid char in byte sequence");
        }
        match base64::decode(b64_content) {
            Ok(content) => Ok(content),
            Err(_) => Err("parse_byte_seq: decoding error"),
        }
    }

    fn parse_number(input_chars: &mut Peekable<Chars>) -> ParseResult<Num> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-number

        let (min_int, max_int) = (-999_999_999_999_999_i64, 999_999_999_999_999_i64);

        let mut is_integer = true;
        let mut sign = 1;
        let mut input_number = String::from("");

        if let Some('-') = input_chars.peek() {
            sign = -1;
            input_chars.next();
        }

        match input_chars.peek() {
            Some(c) if !c.is_ascii_digit() => {
                return Err("parse_number: input number does not start with a digit")
            }
            None => return Err("parse_number: input number lacks a digit"),
            _ => (),
        }

        while let Some(curr_char) = input_chars.peek() {
            match curr_char {
                c if c.is_ascii_digit() => {
                    input_number.push(*curr_char);
                    input_chars.next();
                }
                c if c == &'.' && is_integer => {
                    if input_number.len() > 12 {
                        return Err(
                            "parse_number: decimal too long, illegal position for decimal point",
                        );
                    }
                    input_number.push(*curr_char);
                    is_integer = false;
                    input_chars.next();
                }
                _ => break,
            }

            if is_integer && input_number.len() > 15 {
                return Err("parse_number: integer too long, length > 15");
            }

            if !is_integer && input_number.len() > 16 {
                return Err("parse_number: decimal too long, length > 16");
            }
        }

        if is_integer {
            let output_number = input_number
                .parse::<i64>()
                .map_err(|_err| "parse_number: parsing i64 failed")?
                * sign;

            if output_number < min_int || max_int < output_number {
                return Err("parse_number: integer number is out of range");
            }

            return Ok(Num::Integer(output_number));
        }

        let chars_after_dot = input_number
            .find('.')
            .map(|dot_pos| input_number.len() - dot_pos - 1);
        match chars_after_dot {
            Some(1..=3) => {
                let mut output_number = Decimal::from_str(&input_number)
                    .map_err(|_err| "parse_number: parsing f64 failed")?;

                if sign == -1 {
                    output_number.set_sign_negative(true)
                }

                Ok(Num::Decimal(output_number))
            }
            _ => Err("parse_number: invalid decimal fraction length"),
        }
    }

    fn parse_parameters(input_chars: &mut Peekable<Chars>) -> ParseResult<Parameters> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-param

        let mut params = Parameters::new();
        // expected.insert("str".to_owned(), BareItem::String("param_val".to_owned()));
        // Ok(expected)

        while let Some(curr_char) = input_chars.peek() {
            if curr_char == &';' {
                input_chars.next();
            } else {
                break;
            }

            utils::consume_sp_chars(input_chars);

            let param_name = Self::parse_key(input_chars)?;
            let param_value = match input_chars.peek() {
                Some('=') => {
                    input_chars.next();
                    Self::parse_bare_item(input_chars)?
                }
                _ => BareItem::Boolean(true),
            };
            params.insert(param_name, param_value);
        }

        // If parameters already contains a name param_name (comparing character-for-character), overwrite its value.
        // Note that when duplicate Parameter keys are encountered, this has the effect of ignoring all but the last instance.
        Ok(params)
    }

    fn parse_key(input_chars: &mut Peekable<Chars>) -> ParseResult<String> {
        match input_chars.peek() {
            Some(c) if c == &'*' || c.is_ascii_lowercase() => (),
            _ => return Err("parse_key: first char is not lcalpha or *"),
        }

        let mut output = String::new();
        while let Some(curr_char) = input_chars.peek() {
            if !curr_char.is_ascii_lowercase()
                && !curr_char.is_ascii_digit()
                && !"_-*.".contains(*curr_char)
            {
                return Ok(output);
            }

            output.push(*curr_char);
            input_chars.next();
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::iter::FromIterator;

    #[test]
    fn parse() -> Result<(), Box<dyn Error>> {
        let input = "\"some_value\"".as_bytes();
        let parsed_item = Item(BareItem::String("some_value".to_owned()), Parameters::new());
        let expected = Header::Item(parsed_item);
        assert_eq!(expected, Parser::parse(input, "item")?);

        let input = "12.35;a ".as_bytes();
        let param = Parameters::from_iter(vec![("a".to_owned(), BareItem::Boolean(true))]);
        let expected = Header::Item(Item(Decimal::from_str("12.35")?.into(), param));

        assert_eq!(expected, Parser::parse(input, "item")?);
        Ok(())
    }

    #[test]
    fn parse_errors() -> Result<(), Box<dyn Error>> {
        let input = "\"some_value\" trailing_text".as_bytes();
        assert_eq!(
            Err("parse: trailing text after parsed value"),
            Parser::parse(input, "item")
        );
        assert_eq!(
            Err("parse: unrecognized header type"),
            Parser::parse(input, "invalid_type")
        );
        assert_eq!(
            Err("parse_bare_item: empty item"),
            Parser::parse("".as_bytes(), "item")
        );
        Ok(())
    }

    #[test]
    fn parse_list_of_numbers() -> Result<(), Box<dyn Error>> {
        let mut input = "1,42".chars().peekable();
        let item1 = Item(1.into(), Parameters::new());
        let item2 = Item(42.into(), Parameters::new());
        let expected_list: List = vec![item1.into(), item2.into()];
        assert_eq!(expected_list, Parser::parse_list(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_list_with_multiple_spaces() -> Result<(), Box<dyn Error>> {
        let mut input = "1  ,  42".chars().peekable();
        let item1 = Item(1.into(), Parameters::new());
        let item2 = Item(42.into(), Parameters::new());
        let expected_list: List = vec![item1.into(), item2.into()];
        assert_eq!(expected_list, Parser::parse_list(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_list_of_lists() -> Result<(), Box<dyn Error>> {
        let mut input = "(1 2), (42 43)".chars().peekable();
        let item1 = Item(1.into(), Parameters::new());
        let item2 = Item(2.into(), Parameters::new());
        let item3 = Item(42.into(), Parameters::new());
        let item4 = Item(43.into(), Parameters::new());
        let inner_list_1 = InnerList(vec![item1, item2], Parameters::new());
        let inner_list_2 = InnerList(vec![item3, item4], Parameters::new());
        let expected_list: List = vec![inner_list_1.into(), inner_list_2.into()];
        assert_eq!(expected_list, Parser::parse_list(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_list_empty_inner_list() -> Result<(), Box<dyn Error>> {
        let mut input = "()".chars().peekable();
        let inner_list = InnerList(vec![], Parameters::new());
        let expected_list: List = vec![inner_list.into()];
        assert_eq!(expected_list, Parser::parse_list(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_list_empty() -> Result<(), Box<dyn Error>> {
        let mut input = "".chars().peekable();
        let expected_list: List = vec![];
        assert_eq!(expected_list, Parser::parse_list(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_list_of_lists_with_param_and_spaces() -> Result<(), Box<dyn Error>> {
        let mut input = "(  1  42  ); k=*".chars().peekable();
        let item1 = Item(1.into(), Parameters::new());
        let item2 = Item(42.into(), Parameters::new());
        let inner_list_param =
            Parameters::from_iter(vec![("k".to_owned(), BareItem::Token("*".to_owned()))]);
        let inner_list = InnerList(vec![item1, item2], inner_list_param);
        let expected_list: List = vec![inner_list.into()];
        assert_eq!(expected_list, Parser::parse_list(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_list_of_items_and_lists_with_param() -> Result<(), Box<dyn Error>> {
        let mut input = "12, 14, (a  b); param=\"param_value_1\"".chars().peekable();
        let item1 = Item(12.into(), Parameters::new());
        let item2 = Item(14.into(), Parameters::new());
        let item3 = Item(BareItem::Token("a".to_owned()), Parameters::new());
        let item4 = Item(BareItem::Token("b".to_owned()), Parameters::new());
        let inner_list_param = Parameters::from_iter(vec![(
            "param".to_owned(),
            BareItem::String("param_value_1".to_owned()),
        )]);
        let inner_list = InnerList(vec![item3, item4], inner_list_param);
        let expected_list: List = vec![item1.into(), item2.into(), inner_list.into()];
        assert_eq!(expected_list, Parser::parse_list(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_list_errors() -> Result<(), Box<dyn Error>> {
        let mut input = ",".chars().peekable();
        assert_eq!(
            Err("parse_bare_item: item type can't be identified"),
            Parser::parse_list(&mut input)
        );

        let mut input = "a, b c".chars().peekable();
        assert_eq!(
            Err("parse_list: trailing text after item in list"),
            Parser::parse_list(&mut input)
        );

        let mut input = "a,".chars().peekable();
        assert_eq!(
            Err("parse_list: trailing comma at the end of the list"),
            Parser::parse_list(&mut input)
        );

        let mut input = "a     ,    ".chars().peekable();
        assert_eq!(
            Err("parse_list: trailing comma at the end of the list"),
            Parser::parse_list(&mut input)
        );

        let mut input = "a\t \t ,\t ".chars().peekable();
        assert_eq!(
            Err("parse_list: trailing comma at the end of the list"),
            Parser::parse_list(&mut input)
        );

        let mut input = "a\t\t,\t\t\t".chars().peekable();
        assert_eq!(
            Err("parse_list: trailing comma at the end of the list"),
            Parser::parse_list(&mut input)
        );

        let mut input = "(a b),".chars().peekable();
        assert_eq!(
            Err("parse_list: trailing comma at the end of the list"),
            Parser::parse_list(&mut input)
        );

        let mut input = "(1, 2, (a b)".chars().peekable();
        assert_eq!(
            Err("parse_inner_list: bad delimitation"),
            Parser::parse_list(&mut input)
        );

        Ok(())
    }

    #[test]
    fn parse_inner_list_with_param_and_spaces() -> Result<(), Box<dyn Error>> {
        let mut input = "(c b); a=1".chars().peekable();
        let inner_list_param = Parameters::from_iter(vec![("a".to_owned(), 1.into())]);

        let item1 = Item(BareItem::Token("c".to_owned()), Parameters::new());
        let item2 = Item(BareItem::Token("b".to_owned()), Parameters::new());
        let expected = InnerList(vec![item1, item2], inner_list_param);
        assert_eq!(expected, Parser::parse_inner_list(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_item_int_with_space() -> Result<(), Box<dyn Error>> {
        let mut input = "12 ".chars().peekable();
        assert_eq!(
            Item(12.into(), Parameters::new()),
            Parser::parse_item(&mut input)?
        );
        Ok(())
    }

    #[test]
    fn parse_item_decimal_with_bool_param_and_space() -> Result<(), Box<dyn Error>> {
        let mut input = "12.35;a ".chars().peekable();
        let param = Parameters::from_iter(vec![("a".to_owned(), BareItem::Boolean(true))]);
        assert_eq!(
            Item(Decimal::from_str("12.35")?.into(), param),
            Parser::parse_item(&mut input)?
        );
        Ok(())
    }

    #[test]
    fn parse_item_number_with_param() -> Result<(), Box<dyn Error>> {
        let param = Parameters::from_iter(vec![("a1".to_owned(), BareItem::Token("*".to_owned()))]);
        assert_eq!(
            Item(BareItem::String("12.35".to_owned()), param),
            Parser::parse_item(&mut "\"12.35\";a1=*".chars().peekable())?
        );
        Ok(())
    }

    #[test]
    fn parse_item_errors() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Err("parse_bare_item: empty item"),
            Parser::parse_item(&mut "".chars().peekable())
        );
        Ok(())
    }

    #[test]
    fn parse_dict_empty() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Dictionary::new(),
            Parser::parse_dict(&mut "".chars().peekable())?
        );
        Ok(())
    }

    #[test]
    fn parse_dict_with_spaces_and_params() -> Result<(), Box<dyn Error>> {
        let mut input = "abc=123;a=1;b=2, def=456, ghi=789;q=9;r=\"+w\""
            .chars()
            .peekable();
        let item1_params =
            Parameters::from_iter(vec![("a".to_owned(), 1.into()), ("b".to_owned(), 2.into())]);
        let item3_params = Parameters::from_iter(vec![
            ("q".to_owned(), 9.into()),
            ("r".to_owned(), BareItem::String("+w".to_owned())),
        ]);

        let item1 = Item(123.into(), item1_params);
        let item2 = Item(456.into(), Parameters::new());
        let item3 = Item(789.into(), item3_params);

        let expected_dict = Dictionary::from_iter(vec![
            ("abc".to_owned(), item1.into()),
            ("def".to_owned(), item2.into()),
            ("ghi".to_owned(), item3.into()),
        ]);
        assert_eq!(expected_dict, Parser::parse_dict(&mut input)?);

        Ok(())
    }

    #[test]
    fn parse_dict_empty_value() -> Result<(), Box<dyn Error>> {
        let mut input = "a=()".chars().peekable();
        let inner_list = InnerList(vec![], Parameters::new());
        let expected_dict = Dictionary::from_iter(vec![("a".to_owned(), inner_list.into())]);
        assert_eq!(expected_dict, Parser::parse_dict(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_dict_with_token_param() -> Result<(), Box<dyn Error>> {
        let mut input = "a=1, b;foo=*, c=3".chars().peekable();
        let item2_params =
            Parameters::from_iter(vec![("foo".to_owned(), BareItem::Token("*".to_owned()))]);
        let item1 = Item(1.into(), Parameters::new());
        let item2 = Item(BareItem::Boolean(true), item2_params);
        let item3 = Item(3.into(), Parameters::new());
        let expected_dict = Dictionary::from_iter(vec![
            ("a".to_owned(), item1.into()),
            ("b".to_owned(), item2.into()),
            ("c".to_owned(), item3.into()),
        ]);
        assert_eq!(expected_dict, Parser::parse_dict(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_dict_multiple_spaces() -> Result<(), Box<dyn Error>> {
        // input1, input2, input3 must be parsed into the same structure
        let item1 = Item(1.into(), Parameters::new());
        let item2 = Item(2.into(), Parameters::new());
        let expected_dict = Dictionary::from_iter(vec![
            ("a".to_owned(), item1.into()),
            ("b".to_owned(), item2.into()),
        ]);

        let mut input1 = "a=1 ,  b=2".chars().peekable();
        let mut input2 = "a=1\t,\tb=2".chars().peekable();
        let mut input3 = "a=1, b=2".chars().peekable();
        assert_eq!(expected_dict, Parser::parse_dict(&mut input1)?);
        assert_eq!(expected_dict, Parser::parse_dict(&mut input2)?);
        assert_eq!(expected_dict, Parser::parse_dict(&mut input3)?);

        Ok(())
    }

    #[test]
    fn parse_bare_item() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            BareItem::Boolean(false),
            Parser::parse_bare_item(&mut "?0".chars().peekable())?
        );
        assert_eq!(
            BareItem::String("test string".to_owned()),
            Parser::parse_bare_item(&mut "\"test string\"".chars().peekable())?
        );
        assert_eq!(
            BareItem::Token("*token".to_owned()),
            Parser::parse_bare_item(&mut "*token".chars().peekable())?
        );
        assert_eq!(
            BareItem::ByteSeq("base_64 encoding test".to_owned().into_bytes()),
            Parser::parse_bare_item(&mut ":YmFzZV82NCBlbmNvZGluZyB0ZXN0:".chars().peekable())?
        );
        assert_eq!(
            BareItem::Number(Num::Decimal(Decimal::from_str("-3.55")?)),
            Parser::parse_bare_item(&mut "-3.55".chars().peekable())?
        );
        Ok(())
    }

    #[test]
    fn parse_bare_item_errors() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Err("parse_bare_item: item type can't be identified"),
            Parser::parse_bare_item(&mut "!?0".chars().peekable())
        );
        assert_eq!(
            Err("parse_bare_item: item type can't be identified"),
            Parser::parse_bare_item(&mut "_11abc".chars().peekable())
        );
        assert_eq!(
            Err("parse_bare_item: item type can't be identified"),
            Parser::parse_bare_item(&mut "   ".chars().peekable())
        );
        Ok(())
    }

    #[test]
    fn parse_bool() -> Result<(), Box<dyn Error>> {
        let mut input = "?0gk".chars().peekable();
        assert_eq!(false, Parser::parse_bool(&mut input)?);
        assert_eq!(input.collect::<String>(), "gk");

        assert_eq!(false, Parser::parse_bool(&mut "?0".chars().peekable())?);
        assert_eq!(true, Parser::parse_bool(&mut "?1".chars().peekable())?);
        Ok(())
    }

    #[test]
    fn parse_bool_errors() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Err("parse_bool: first char is not '?'"),
            Parser::parse_bool(&mut "".chars().peekable())
        );
        assert_eq!(
            Err("parse_bool: invalid variant"),
            Parser::parse_bool(&mut "?".chars().peekable())
        );
        Ok(())
    }

    #[test]
    fn parse_string() -> Result<(), Box<dyn Error>> {
        let mut input = "\"some string\" ;not string".chars().peekable();
        assert_eq!("some string".to_owned(), Parser::parse_string(&mut input)?);
        assert_eq!(input.collect::<String>(), " ;not string");

        assert_eq!(
            "test".to_owned(),
            Parser::parse_string(&mut "\"test\"".chars().peekable())?
        );
        assert_eq!(
            "".to_owned(),
            Parser::parse_string(&mut "\"\"".chars().peekable())?
        );
        assert_eq!(
            "some string".to_owned(),
            Parser::parse_string(&mut "\"some string\"".chars().peekable())?
        );
        Ok(())
    }

    #[test]
    fn parse_string_errors() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Err("parse_string: first char is not '\"'"),
            Parser::parse_string(&mut "test".chars().peekable())
        );
        assert_eq!(
            Err("parse_string: last input char is '\\'"),
            Parser::parse_string(&mut "\"\\".chars().peekable())
        );
        assert_eq!(
            Err("parse_string: disallowed char after '\\'"),
            Parser::parse_string(&mut "\"\\l\"".chars().peekable())
        );
        assert_eq!(
            Err("parse_string: not a visible char"),
            Parser::parse_string(&mut "\"\u{1f}\"".chars().peekable())
        );
        assert_eq!(
            Err("parse_string: no closing '\"'"),
            Parser::parse_string(&mut "\"smth".chars().peekable())
        );
        Ok(())
    }

    #[test]
    fn parse_token() -> Result<(), Box<dyn Error>> {
        let mut input = "*some:token}not token".chars().peekable();
        assert_eq!("*some:token".to_owned(), Parser::parse_token(&mut input)?);
        assert_eq!(input.collect::<String>(), "}not token");

        assert_eq!(
            "token".to_owned(),
            Parser::parse_token(&mut "token".chars().peekable())?
        );
        assert_eq!(
            "a_b-c.d3:f%00/*".to_owned(),
            Parser::parse_token(&mut "a_b-c.d3:f%00/*".chars().peekable())?
        );
        assert_eq!(
            "TestToken".to_owned(),
            Parser::parse_token(&mut "TestToken".chars().peekable())?
        );
        assert_eq!(
            "some".to_owned(),
            Parser::parse_token(&mut "some@token".chars().peekable())?
        );
        assert_eq!(
            "*TestToken*".to_owned(),
            Parser::parse_token(&mut "*TestToken*".chars().peekable())?
        );
        assert_eq!(
            "*".to_owned(),
            Parser::parse_token(&mut "*[@:token".chars().peekable())?
        );
        assert_eq!(
            "test".to_owned(),
            Parser::parse_token(&mut "test token".chars().peekable())?
        );

        Ok(())
    }

    #[test]
    fn parse_token_errors() -> Result<(), Box<dyn Error>> {
        let mut input = "765token".chars().peekable();
        assert_eq!(
            Err("parse_token: first char is not ALPHA or '*'"),
            Parser::parse_token(&mut input)
        );
        assert_eq!(input.collect::<String>(), "765token");

        assert_eq!(
            Err("parse_token: first char is not ALPHA or '*'"),
            Parser::parse_token(&mut "7token".chars().peekable())
        );
        assert_eq!(
            Err("parse_token: empty input string"),
            Parser::parse_token(&mut "".chars().peekable())
        );
        Ok(())
    }

    #[test]
    fn parse_byte_sequence() -> Result<(), Box<dyn Error>> {
        let mut input = ":aGVsbG8:rest_of_str".chars().peekable();
        assert_eq!(
            "hello".to_owned().into_bytes(),
            Parser::parse_byte_sequence(&mut input)?
        );
        assert_eq!("rest_of_str", input.collect::<String>());

        assert_eq!(
            "hello".to_owned().into_bytes(),
            Parser::parse_byte_sequence(&mut ":aGVsbG8:".chars().peekable())?
        );
        assert_eq!(
            "test_encode".to_owned().into_bytes(),
            Parser::parse_byte_sequence(&mut ":dGVzdF9lbmNvZGU:".chars().peekable())?
        );
        assert_eq!(
            "new:year tree".to_owned().into_bytes(),
            Parser::parse_byte_sequence(&mut ":bmV3OnllYXIgdHJlZQ==:".chars().peekable())?
        );
        assert_eq!(
            "".to_owned().into_bytes(),
            Parser::parse_byte_sequence(&mut "::".chars().peekable())?
        );
        Ok(())
    }

    #[test]
    fn parse_byte_sequence_errors() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Err("parse_byte_seq: first char is not ':'"),
            Parser::parse_byte_sequence(&mut "aGVsbG8".chars().peekable())
        );
        assert_eq!(
            Err("parse_byte_seq: invalid char in byte sequence"),
            Parser::parse_byte_sequence(&mut ":aGVsb G8=:".chars().peekable())
        );
        assert_eq!(
            Err("parse_byte_seq: no closing ':'"),
            Parser::parse_byte_sequence(&mut ":aGVsbG8=".chars().peekable())
        );
        Ok(())
    }

    #[test]
    fn parse_number_int() -> Result<(), Box<dyn Error>> {
        let mut input = "-733333333332d.14".chars().peekable();
        assert_eq!(
            Num::Integer(-733333333332),
            Parser::parse_number(&mut input)?
        );
        assert_eq!("d.14", input.collect::<String>());

        assert_eq!(
            Num::Integer(42),
            Parser::parse_number(&mut "42".chars().peekable())?
        );
        assert_eq!(
            Num::Integer(-42),
            Parser::parse_number(&mut "-42".chars().peekable())?
        );
        assert_eq!(
            Num::Integer(-42),
            Parser::parse_number(&mut "-042".chars().peekable())?
        );
        assert_eq!(
            Num::Integer(0),
            Parser::parse_number(&mut "0".chars().peekable())?
        );
        assert_eq!(
            Num::Integer(0),
            Parser::parse_number(&mut "00".chars().peekable())?
        );
        assert_eq!(
            Num::Integer(123456789012345),
            Parser::parse_number(&mut "123456789012345".chars().peekable())?
        );
        assert_eq!(
            Num::Integer(-123456789012345),
            Parser::parse_number(&mut "-123456789012345".chars().peekable())?
        );
        assert_eq!(
            Num::Integer(2),
            Parser::parse_number(&mut "2,3".chars().peekable())?
        );
        assert_eq!(
            Num::Integer(4),
            Parser::parse_number(&mut "4-2".chars().peekable())?
        );
        assert_eq!(
            Num::Integer(-999999999999999),
            Parser::parse_number(&mut "-999999999999999".chars().peekable())?
        );
        assert_eq!(
            Num::Integer(999999999999999),
            Parser::parse_number(&mut "999999999999999".chars().peekable())?
        );

        Ok(())
    }

    #[test]
    fn parse_number_decimal() -> Result<(), Box<dyn Error>> {
        let mut input = "00.42 test string".chars().peekable();
        assert_eq!(
            Num::Decimal(Decimal::from_str("0.42")?),
            Parser::parse_number(&mut input)?
        );
        assert_eq!(" test string", input.collect::<String>());

        assert_eq!(
            Num::Decimal(Decimal::from_str("1.5")?),
            Parser::parse_number(&mut "1.5.4.".chars().peekable())?
        );
        assert_eq!(
            Num::Decimal(Decimal::from_str("1.8")?),
            Parser::parse_number(&mut "1.8.".chars().peekable())?
        );
        assert_eq!(
            Num::Decimal(Decimal::from_str("1.7")?),
            Parser::parse_number(&mut "1.7.0".chars().peekable())?
        );
        assert_eq!(
            Num::Decimal(Decimal::from_str("3.14")?),
            Parser::parse_number(&mut "3.14".chars().peekable())?
        );
        assert_eq!(
            Num::Decimal(Decimal::from_str("-3.14")?),
            Parser::parse_number(&mut "-3.14".chars().peekable())?
        );
        assert_eq!(
            Num::Decimal(Decimal::from_str("123456789012.1")?),
            Parser::parse_number(&mut "123456789012.1".chars().peekable())?
        );
        assert_eq!(
            Num::Decimal(Decimal::from_str("1234567890.112")?),
            Parser::parse_number(&mut "1234567890.112".chars().peekable())?
        );

        Ok(())
    }

    #[test]
    fn parse_number_errors() -> Result<(), Box<dyn Error>> {
        let mut input = ":aGVsbG8:rest".chars().peekable();
        assert_eq!(
            Err("parse_number: input number does not start with a digit"),
            Parser::parse_number(&mut input)
        );
        assert_eq!(":aGVsbG8:rest", input.collect::<String>());

        let mut input = "-11.5555 test string".chars().peekable();
        assert_eq!(
            Err("parse_number: invalid decimal fraction length"),
            Parser::parse_number(&mut input)
        );
        assert_eq!(" test string", input.collect::<String>());

        assert_eq!(
            Err("parse_number: input number does not start with a digit"),
            Parser::parse_number(&mut "--0".chars().peekable())
        );
        assert_eq!(
            Err("parse_number: integer too long, length > 15"),
            Parser::parse_number(&mut "1999999999999999".chars().peekable())
        );
        assert_eq!(
            Err("parse_number: input number does not start with a digit"),
            Parser::parse_number(&mut "- 42".chars().peekable())
        );
        assert_eq!(
            Err("parse_number: input number does not start with a digit"),
            Parser::parse_number(&mut "- 42".chars().peekable())
        );
        assert_eq!(
            Err("parse_number: invalid decimal fraction length"),
            Parser::parse_number(&mut "1..4".chars().peekable())
        );
        assert_eq!(
            Err("parse_number: input number lacks a digit"),
            Parser::parse_number(&mut "-".chars().peekable())
        );
        assert_eq!(
            Err("parse_number: invalid decimal fraction length"),
            Parser::parse_number(&mut "-5. 14".chars().peekable())
        );
        assert_eq!(
            Err("parse_number: invalid decimal fraction length"),
            Parser::parse_number(&mut "7. 1".chars().peekable())
        );
        assert_eq!(
            Err("parse_number: invalid decimal fraction length"),
            Parser::parse_number(&mut "-7.3333333333".chars().peekable())
        );
        assert_eq!(
            Err("parse_number: decimal too long, illegal position for decimal point"),
            Parser::parse_number(&mut "-7333333333323.12".chars().peekable())
        );

        Ok(())
    }

    #[test]
    fn parse_parameters_string() -> Result<(), Box<dyn Error>> {
        let mut input = ";b=\"param_val\"".chars().peekable();
        let expected = Parameters::from_iter(vec![(
            "b".to_owned(),
            BareItem::String("param_val".to_owned()),
        )]);
        assert_eq!(expected, Parser::parse_parameters(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_parameters_bool() -> Result<(), Box<dyn Error>> {
        let mut input = ";b;a".chars().peekable();
        let expected = Parameters::from_iter(vec![
            ("b".to_owned(), BareItem::Boolean(true)),
            ("a".to_owned(), BareItem::Boolean(true)),
        ]);
        assert_eq!(expected, Parser::parse_parameters(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_parameters_mixed_types() -> Result<(), Box<dyn Error>> {
        let mut input = ";key1=?0;key2=746.15".chars().peekable();
        let expected = Parameters::from_iter(vec![
            ("key1".to_owned(), BareItem::Boolean(false)),
            ("key2".to_owned(), Decimal::from_str("746.15")?.into()),
        ]);
        assert_eq!(expected, Parser::parse_parameters(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_parameters_with_spaces() -> Result<(), Box<dyn Error>> {
        let mut input = "; key1=?0; key2=11111".chars().peekable();
        let expected = Parameters::from_iter(vec![
            ("key1".to_owned(), BareItem::Boolean(false)),
            ("key2".to_owned(), 11111.into()),
        ]);
        assert_eq!(expected, Parser::parse_parameters(&mut input)?);
        Ok(())
    }

    #[test]
    fn parse_parameters_empty() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Parameters::new(),
            Parser::parse_parameters(&mut " key1=?0; key2=11111".chars().peekable())?
        );
        assert_eq!(
            Parameters::new(),
            Parser::parse_parameters(&mut "".chars().peekable())?
        );
        assert_eq!(
            Parameters::new(),
            Parser::parse_parameters(&mut "[;a=1".chars().peekable())?
        );
        assert_eq!(
            Parameters::new(),
            Parser::parse_parameters(&mut String::new().chars().peekable())?
        );
        Ok(())
    }

    #[test]
    fn parse_key() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            "a".to_owned(),
            Parser::parse_key(&mut "a=1".chars().peekable())?
        );
        assert_eq!(
            "a1".to_owned(),
            Parser::parse_key(&mut "a1=10".chars().peekable())?
        );
        assert_eq!(
            "*1".to_owned(),
            Parser::parse_key(&mut "*1=10".chars().peekable())?
        );
        assert_eq!(
            "f".to_owned(),
            Parser::parse_key(&mut "f[f=10".chars().peekable())?
        );
        Ok(())
    }

    #[test]
    fn parse_key_errors() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Err("parse_key: first char is not lcalpha or *"),
            Parser::parse_key(&mut "[*f=10".chars().peekable())
        );
        Ok(())
    }
}
