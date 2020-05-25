use crate::utils;
use indexmap::IndexMap;
use rust_decimal::prelude::*;
use std::iter::Peekable;
use std::str::Chars;

type InnerList = Vec<Item>;
type Dictionary = IndexMap<String, DictionaryValue>;
type Parameters = IndexMap<String, BareItem>;

#[derive(Debug, PartialEq)]
enum DictionaryValue {
    Item(Item),
    InnerList(InnerList),
}

#[derive(Debug, PartialEq)]
struct Item {
    bare_item: BareItem,
    parameters: Parameters,
}

#[derive(Debug, PartialEq)]
enum Num {
    Decimal(Decimal),
    Integer(i64),
}

#[derive(Debug, PartialEq)]
enum BareItem {
    Number(Num),
    String(String),
    ByteSeq(Vec<u8>),
    Boolean(bool),
    Token(String),
}

#[derive(Debug)]
struct Parser {
    input_str: String,
}

impl Parser {
    // fn new(input_string: String) -> Self {
    //     Parser {
    //         input_str: input_string,
    //     }
    // }
    //
    // fn parse(self, header_type: &str) -> Result<Header, &str> {
    //     // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#text-parse
    //     let mut input = self.input_str.chars().peekable();
    //     match header_type {
    //         "list" => Ok(Header(Self::parse_list(&mut input)?),
    //         "dict" => Ok(Header(Self::parse_dict(&mut input)?),
    //         "item" => Ok(Header(Self::parse_item(&mut input)?),
    //     }
    // }

    fn parse_item(input_chars: &mut Peekable<Chars>) -> Result<Item, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-item

        let bare_item = Self::parse_bare_item(input_chars)?;
        let parameters = Self::parse_parameters(input_chars)?;

        Ok(Item {
            bare_item,
            parameters,
        })
    }

    fn parse_bare_item(mut input: &mut Peekable<Chars>) -> Result<BareItem, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-bare-item

        match input.peek() {
            Some(&'?') => Ok(BareItem::Boolean(Self::parse_bool(&mut input)?)),
            Some(&'"') => Ok(BareItem::String(Self::parse_string(&mut input)?)),
            Some(&':') => Ok(BareItem::ByteSeq(Self::parse_byte_sequence(&mut input)?)),
            Some(&c) if c == '*' || c.is_ascii_alphabetic() => {
                Ok(BareItem::Token(Self::parse_token(&mut input)?))
            }
            Some(&c) if c == '-' || c.is_ascii_digit() => {
                Ok(BareItem::Number(Self::parse_number(&mut input)?))
            }
            _ => Err("parse_bare_item: item type can't be identified"),
        }
    }

    fn parse_bool(input: &mut Peekable<Chars>) -> Result<bool, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-boolean

        if input.next() != Some('?') {
            return Err("parse_bool: first char is not '?'");
        }

        match input.next() {
            Some('0') => Ok(false),
            Some('1') => Ok(true),
            _ => Err("parse_bool: invalid variant"),
        }
    }

    fn parse_string(input: &mut Peekable<Chars>) -> Result<String, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-string

        if input.next() != Some('\"') {
            return Err("parse_string: first char is not '\"'");
        }

        let mut output_string = String::from("");
        while let Some(curr_char) = input.next() {
            match curr_char {
                '\"' => return Ok(output_string),
                '\x7f' | '\x00'..='\x1f' => return Err("parse_string: not a visible char"),
                '\\' => match input.next() {
                    Some('\\') => {
                        output_string.push(curr_char);
                    }
                    Some('\"') => {
                        output_string.push(curr_char);
                    }
                    None => return Err("parse_string: no chars after '\\'"),
                    _ => return Err("parse_string: invalid char after '\\'"),
                },
                _ => output_string.push(curr_char),
            }
        }
        Err("parse_string: no closing '\"'")
    }

    fn parse_token(input: &mut Peekable<Chars>) -> Result<String, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-token

        if let Some(first_char) = input.peek() {
            if !first_char.is_ascii_alphabetic() && first_char != &'*' {
                return Err("parse_token: first char is not ALPHA or '*'");
            }
        } else {
            return Err("parse_token: empty input string");
        }

        let mut output_string = String::from("");
        while let Some(curr_char) = input.peek() {
            if !utils::is_tchar(*curr_char) && curr_char != &':' && curr_char != &'/' {
                return Ok(output_string);
            }

            match input.next() {
                Some(c) => output_string.push(c),
                None => return Err("parse_token: end of the string"),
            }
        }
        Ok(output_string)
    }

    fn parse_byte_sequence(input: &mut Peekable<Chars>) -> Result<Vec<u8>, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-binary

        if input.next() != Some(':') {
            return Err("parse_byte_seq: first char is not ':'");
        }

        if !input.clone().any(|c| c == ':') {
            return Err("parse_byte_seq: no closing ':'");
        }
        let b64_content = input.take_while(|c| c != &':').collect::<String>();
        if !b64_content
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '=' || c == '/')
        {
            return Err("parse_byte_seq: invalid char in byte sequence");
        }
        match base64::decode(b64_content) {
            Ok(content) => Ok(content),
            Err(_) => Err("parse_byte_seq: decoding error"),
        }
    }

    fn parse_number(input: &mut Peekable<Chars>) -> Result<Num, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-number

        let (min_int, max_int) = (-999_999_999_999_999_i64, 999_999_999_999_999_i64);

        let mut is_integer = true;
        let mut sign = 1;
        let mut input_number = String::from("");

        if let Some('-') = input.peek() {
            sign = -1;
            input.next();
        }

        match input.peek() {
            Some(c) if !c.is_ascii_digit() => {
                return Err("parse_number: input number does not start with a digit")
            }
            None => return Err("parse_number: input number lacks a digit"),
            _ => (),
        }

        while let Some(curr_char) = input.peek() {
            match curr_char {
                c if c.is_ascii_digit() => {
                    input_number.push(*curr_char);
                    input.next();
                }
                c if c == &'.' && is_integer => {
                    if input_number.len() > 12 {
                        return Err(
                            "parse_number: decimal too long, illegal position for decimal point",
                        );
                    }
                    input_number.push(*curr_char);
                    is_integer = false;
                    input.next();
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
            Some(1) | Some(2) => {
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

    fn parse_parameters(input: &mut Peekable<Chars>) -> Result<Parameters, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-param

        let mut params = Parameters::new();
        // expected.insert("str".to_owned(), BareItem::String("param_val".to_owned()));
        // Ok(expected)

        while let Some(curr_char) = input.peek() {
            if curr_char == &';' {
                input.next();
            } else {
                break;
            }

            match input.peek() {
                Some(c) if c.is_whitespace() => {
                    input.next();
                }
                _ => (),
            }

            let param_name = Self::parse_key(input)?;
            let param_value = match input.peek() {
                Some('=') => {
                    input.next();
                    Self::parse_bare_item(input)?
                }
                _ => BareItem::Boolean(true),
            };
            params.insert(param_name, param_value);
        }

        // If parameters already contains a name param_name (comparing character-for-character), overwrite its value.
        // Note that when duplicate Parameter keys are encountered, this has the effect of ignoring all but the last instance.
        Ok(params)
    }

    fn parse_key(input: &mut Peekable<Chars>) -> Result<String, &'static str> {
        match input.peek() {
            Some(c) if c == &'*' || c.is_ascii_lowercase() => (),
            _ => return Err("parse_key: first char is not lcalpha or *"),
        }

        let mut output = String::new();
        while let Some(curr_char) = input.peek() {
            if !curr_char.is_ascii_lowercase()
                && !curr_char.is_ascii_digit()
                && !"_-*.".contains(*curr_char)
            {
                return Ok(output);
            }

            output.push(*curr_char);
            input.next();
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn parse_item() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Item {
                bare_item: BareItem::Number(Num::Integer(12)),
                parameters: Parameters::new()
            },
            Parser::parse_item(&mut "12 ".chars().peekable())?
        );

        let mut param = Parameters::new();
        param.insert("a".to_owned(), BareItem::Boolean(true));

        assert_eq!(
            Item {
                bare_item: BareItem::Number(Num::Decimal(Decimal::from_str("12.35")?)),
                parameters: param
            },
            Parser::parse_item(&mut "12.35;a ".chars().peekable())?
        );

        let mut param = Parameters::new();
        param.insert("a1".to_owned(), BareItem::Token("*".to_owned()));
        assert_eq!(
            Item {
                bare_item: BareItem::String("12.35".to_owned()),
                parameters: param
            },
            Parser::parse_item(&mut "\"12.35\";a1=*".chars().peekable())?
        );

        assert_eq!(
            Err("parse_bare_item: item type can't be identified"),
            Parser::parse_item(&mut "".chars().peekable())
        );

        Ok(())
    }

    #[test]
    fn parse_dict() -> Result<(), Box<dyn Error>> {
        assert_eq!(1, 1);
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
        assert_eq!(
            Err("parse_string: first char is not '\"'"),
            Parser::parse_string(&mut "test".chars().peekable())
        );
        assert_eq!(
            Err("parse_string: no chars after '\\'"),
            Parser::parse_string(&mut "\"\\".chars().peekable())
        );
        assert_eq!(
            Err("parse_string: invalid char after '\\'"),
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

        let mut input = "765token".chars().peekable();
        assert_eq!(
            Err("parse_token: first char is not ALPHA or '*'"),
            Parser::parse_token(&mut input)
        );
        assert_eq!(input.collect::<String>(), "765token");

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
    fn parse_number() -> Result<(), Box<dyn Error>> {
        let mut input = "-733333333332d.14".chars().peekable();
        assert_eq!(
            Num::Integer(-733333333332),
            Parser::parse_number(&mut input)?
        );
        assert_eq!("d.14", input.collect::<String>());

        let mut input = "00.42 test string".chars().peekable();
        assert_eq!(
            Num::Decimal(Decimal::from_str("0.42")?),
            Parser::parse_number(&mut input)?
        );
        assert_eq!(" test string", input.collect::<String>());

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
        assert_eq!(
            Err("parse_number: invalid decimal fraction length"),
            Parser::parse_number(&mut "-733333333332.124".chars().peekable())
        );
        Ok(())
    }

    #[test]
    fn parse_parameters() -> Result<(), Box<dyn Error>> {
        let mut expected = Parameters::new();
        expected.insert("b".to_owned(), BareItem::String("param_val".to_owned()));
        assert_eq!(
            expected,
            Parser::parse_parameters(&mut ";b=\"param_val\"".chars().peekable())?
        );

        let mut expected = Parameters::new();
        expected.insert("b".to_owned(), BareItem::Boolean(true));
        expected.insert("a".to_owned(), BareItem::Boolean(true));
        assert_eq!(
            expected,
            Parser::parse_parameters(&mut ";b;a".chars().peekable())?
        );

        let mut expected = Parameters::new();
        expected.insert("key1".to_owned(), BareItem::Boolean(false));
        expected.insert(
            "key2".to_owned(),
            BareItem::Number(Num::Decimal(Decimal::from_str("746.15")?)),
        );
        assert_eq!(
            expected,
            Parser::parse_parameters(&mut ";key1=?0;key2=746.15".chars().peekable())?
        );

        let mut expected = Parameters::new();
        expected.insert("key1".to_owned(), BareItem::Boolean(false));
        expected.insert("key2".to_owned(), BareItem::Number(Num::Integer(11111)));
        assert_eq!(
            expected,
            Parser::parse_parameters(&mut "; key1=?0; key2=11111".chars().peekable())?
        );

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
        assert_eq!(
            Err("parse_key: first char is not lcalpha or *"),
            Parser::parse_key(&mut "[*f=10".chars().peekable())
        );
        Ok(())
    }
}
