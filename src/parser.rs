use crate::utils;
use indexmap::IndexMap;
use std::iter::Peekable;
use std::str::Chars;

type Parameters = IndexMap<String, BareItem>;

#[derive(Debug)]
struct Item {
    bare_item: BareItem,
    parameters: Option<Parameters>,
}

#[derive(Debug, PartialEq)]
enum Num {
    Decimal(f64), // Need to change it later to smth more precise
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

    // fn parse_item(self) -> Result<Item, ()> {
    // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-item
    //     // parse item
    //     // parse parameters
    //     // return Item { ... }
    //     let chars_iter = self.input_str.chars().peekable();
    //     let bare_item = Self::parse_bare_item(chars_iter)?;
    //     Ok(Item {
    //         bare_item,
    //         parameters: None,
    //     })
    // }

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
            _ => Err("parse_bare_item: item type is unrecognized"),
        }
    }

    fn parse_bool(input: &mut Peekable<Chars>) -> Result<bool, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-boolean

        if input.next() != Some('?') {
            return Err("bool: first char is not '?'");
        }

        match input.next() {
            Some('0') => Ok(false),
            Some('1') => Ok(true),
            _ => Err("bool: invalid variant"),
        }
    }

    fn parse_string(input: &mut Peekable<Chars>) -> Result<String, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-string

        if input.next() != Some('\"') {
            return Err("string: first char is not '\"'");
        }

        let mut output_string = String::from("");
        while let Some(curr_char) = input.next() {
            if curr_char == '\\' {
                match input.next() {
                    Some('\\') => output_string.push(curr_char),
                    Some('\"') => output_string.push(curr_char),
                    None => return Err("string: no chars after '\\'"),
                    _ => return Err("string: invalid char after '\\'"),
                }
            } else if (curr_char >= '\x00' && curr_char <= '\x1f') || curr_char == '\x7f' {
                return Err("string: not a visible char");
            } else if curr_char == '\"' {
                return Ok(output_string);
            } else {
                output_string.push(curr_char);
            }
        }
        Err("string: no closing '\"'")
    }

    fn parse_token(input: &mut Peekable<Chars>) -> Result<String, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-token

        if let Some(first_char) = input.peek() {
            if !first_char.is_ascii_alphabetic() && first_char != &'*' {
                return Err("token: first char is not ALPHA or '*'");
            }
        } else {
            return Err("token: empty input string");
        }
        let mut output_string = String::from("");
        while let Some(curr_char) = input.peek() {
            if !utils::is_tchar(*curr_char) && curr_char != &':' && curr_char != &'/' {
                return Ok(output_string);
            }

            match input.next() {
                Some(c) => output_string.push(c),
                None => return Err("token: end of the string"),
            }
        }
        Ok(output_string)
    }

    fn parse_byte_sequence(input: &mut Peekable<Chars>) -> Result<Vec<u8>, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-binary

        if input.next() != Some(':') {
            return Err("byte_seq: first char is not ':'");
        }

        if !input.clone().any(|c| c == ':') {
            return Err("byte_seq: no closing ':'");
        }
        let b64_content = input.take_while(|c| c != &':').collect::<String>();
        if !b64_content
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '=' || c == '/')
        {
            return Err("byte_seq: invalid char in byte sequence");
        }
        match base64::decode(b64_content) {
            Ok(content) => Ok(content),
            Err(_) => Err("byte_seq: decoding error"),
        }
    }

    fn parse_number(input: &mut Peekable<Chars>) -> Result<Num, &'static str> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-number

        let (min_int, max_int) = (-999_999_999_999_999_i64, 999_999_999_999_999_i64);

        let mut num_type = "int";
        let mut sign = 1;
        let mut input_number = String::from("");

        if let Some('-') = input.peek() {
            input.next();
            sign = -1;
        }

        match input.peek() {
            Some(c) if c.is_ascii_digit() => {
                input_number.push(*c);
                input.next();
            }
            None => return Err("parse_number: empty integer"),
            _ => return Err("parse_error: number does not start with digit"),
        }

        while let Some(curr_char) = input.peek() {
            if curr_char.is_ascii_digit() {
                input_number.push(*curr_char)
            } else if num_type == "int" && curr_char == &'.' {
                if input_number.len() > 12 {
                    return Err("parse_number: input_number length > 12");
                }
                input_number.push(*curr_char);
                num_type = "decimal";
            } else {
                break;
            }

            input.next();
            if num_type == "int" && input_number.len() > 15 {
                println!("VALUE: {}", input_number);
                return Err("parse_number: int - input_number length > 15 characters");
            }

            if num_type == "decimal" && input_number.len() > 16 {
                return Err("parse_number: decimal - input_number length > 15 characters");
            }
        }

        if num_type == "int" {
            let output_number = input_number.parse::<i64>().unwrap() * sign;
            if output_number < max_int && output_number > min_int {
                Ok(Num::Integer(output_number))
            } else {
                Err("parse_number: int - input_number is out of range")
            }
        } else if num_type == "decimal" {
            let chars_after_dot = input_number.len() - input_number.find('.').unwrap() - 1;
            match chars_after_dot {
                1 | 2 => {
                    let output_number = input_number.parse::<f64>().unwrap() * sign as f64;
                    Ok(Num::Decimal(output_number))
                }
                _ => Err("parse_number: invalid decimal fraction length"),
            }
        } else {
            Err("parse_number: unknown error")
        }
    }

    fn parse_parameters() -> Result<(), ()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-param

        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bare_item() {
        assert_eq!(
            Ok(BareItem::Boolean(false)),
            Parser::parse_bare_item(&mut "?0".chars().peekable())
        );
        assert_eq!(
            Ok(BareItem::String("test string".to_owned())),
            Parser::parse_bare_item(&mut "\"test string\"".chars().peekable())
        );
        assert_eq!(
            Ok(BareItem::Token("*token".to_owned())),
            Parser::parse_bare_item(&mut "*token".chars().peekable())
        );
        assert_eq!(
            Ok(BareItem::ByteSeq(
                "base_64 encoding test".to_owned().into_bytes()
            )),
            Parser::parse_bare_item(&mut ":YmFzZV82NCBlbmNvZGluZyB0ZXN0:".chars().peekable())
        );
        assert_eq!(
            Ok(BareItem::Number(Num::Decimal(-3.55))),
            Parser::parse_bare_item(&mut "-3.55".chars().peekable())
        );

        assert_eq!(
            Err("parse_bare_item: item type is unrecognized"),
            Parser::parse_bare_item(&mut "!?0".chars().peekable())
        );
        assert_eq!(
            Err("parse_bare_item: item type is unrecognized"),
            Parser::parse_bare_item(&mut "_11abc".chars().peekable())
        );
        assert_eq!(
            Err("parse_bare_item: item type is unrecognized"),
            Parser::parse_bare_item(&mut "   ".chars().peekable())
        );
    }

    #[test]
    fn parse_bool() {
        let mut input = "?0gk".chars().peekable();
        assert_eq!(false, Parser::parse_bool(&mut input).unwrap());
        assert_eq!(input.collect::<String>(), "gk");

        assert_eq!(
            false,
            Parser::parse_bool(&mut "?0".chars().peekable()).unwrap()
        );
        assert_eq!(
            true,
            Parser::parse_bool(&mut "?1".chars().peekable()).unwrap()
        );

        assert_eq!(
            Err("bool: first char is not '?'"),
            Parser::parse_bool(&mut "".chars().peekable())
        );
        assert_eq!(
            Err("bool: invalid variant"),
            Parser::parse_bool(&mut "?".chars().peekable())
        );
    }

    #[test]
    fn parse_string() {
        let mut input = "\"some string\" ;not string".chars().peekable();
        assert_eq!(
            "some string".to_owned(),
            Parser::parse_string(&mut input).unwrap()
        );
        assert_eq!(input.collect::<String>(), " ;not string");

        assert_eq!(
            "test".to_owned(),
            Parser::parse_string(&mut "\"test\"".chars().peekable()).unwrap()
        );
        assert_eq!(
            "".to_owned(),
            Parser::parse_string(&mut "\"\"".chars().peekable()).unwrap()
        );
        assert_eq!(
            "some string".to_owned(),
            Parser::parse_string(&mut "\"some string\"".chars().peekable()).unwrap()
        );

        assert_eq!(
            Err("string: first char is not '\"'"),
            Parser::parse_string(&mut "test".chars().peekable())
        );
        assert_eq!(
            Err("string: no chars after '\\'"),
            Parser::parse_string(&mut "\"\\".chars().peekable())
        );
        assert_eq!(
            Err("string: invalid char after '\\'"),
            Parser::parse_string(&mut "\"\\l\"".chars().peekable())
        );
        assert_eq!(
            Err("string: not a visible char"),
            Parser::parse_string(&mut "\"\u{1f}\"".chars().peekable())
        );
        assert_eq!(
            Err("string: no closing '\"'"),
            Parser::parse_string(&mut "\"smth".chars().peekable())
        );
    }

    #[test]
    fn parse_token() {
        let mut input = "*some:token}not token".chars().peekable();
        assert_eq!(
            "*some:token".to_owned(),
            Parser::parse_token(&mut input).unwrap()
        );
        assert_eq!(input.collect::<String>(), "}not token");

        let mut input = "765token".chars().peekable();
        assert_eq!(
            Err("token: first char is not ALPHA or '*'"),
            Parser::parse_token(&mut input)
        );
        assert_eq!(input.collect::<String>(), "765token");

        assert_eq!(
            "token".to_owned(),
            Parser::parse_token(&mut "token".chars().peekable()).unwrap()
        );

        assert_eq!(
            "a_b-c.d3:f%00/*".to_owned(),
            Parser::parse_token(&mut "a_b-c.d3:f%00/*".chars().peekable()).unwrap()
        );
        assert_eq!(
            "TestToken".to_owned(),
            Parser::parse_token(&mut "TestToken".chars().peekable()).unwrap()
        );
        assert_eq!(
            "some".to_owned(),
            Parser::parse_token(&mut "some@token".chars().peekable()).unwrap()
        );
        assert_eq!(
            "*TestToken*".to_owned(),
            Parser::parse_token(&mut "*TestToken*".chars().peekable()).unwrap()
        );
        assert_eq!(
            "*".to_owned(),
            Parser::parse_token(&mut "*[@:token".chars().peekable()).unwrap()
        );
        assert_eq!(
            "test".to_owned(),
            Parser::parse_token(&mut "test token".chars().peekable()).unwrap()
        );
        assert_eq!(
            Err("token: first char is not ALPHA or '*'"),
            Parser::parse_token(&mut "7token".chars().peekable())
        );
        assert_eq!(
            Err("token: empty input string"),
            Parser::parse_token(&mut "".chars().peekable())
        );
    }

    #[test]
    fn parse_byte_sequence() {
        let mut input = ":aGVsbG8:rest_of_str".chars().peekable();
        assert_eq!(
            "hello".to_owned().into_bytes(),
            Parser::parse_byte_sequence(&mut input).unwrap()
        );
        assert_eq!("rest_of_str", input.collect::<String>());

        assert_eq!(
            "hello".to_owned().into_bytes(),
            Parser::parse_byte_sequence(&mut ":aGVsbG8:".chars().peekable()).unwrap()
        );
        assert_eq!(
            "test_encode".to_owned().into_bytes(),
            Parser::parse_byte_sequence(&mut ":dGVzdF9lbmNvZGU:".chars().peekable()).unwrap()
        );
        assert_eq!(
            "new:year tree".to_owned().into_bytes(),
            Parser::parse_byte_sequence(&mut ":bmV3OnllYXIgdHJlZQ==:".chars().peekable()).unwrap()
        );
        assert_eq!(
            "".to_owned().into_bytes(),
            Parser::parse_byte_sequence(&mut "::".chars().peekable()).unwrap()
        );
        assert_eq!(
            Err("byte_seq: first char is not ':'"),
            Parser::parse_byte_sequence(&mut "aGVsbG8".chars().peekable())
        );
        assert_eq!(
            Err("byte_seq: invalid char in byte sequence"),
            Parser::parse_byte_sequence(&mut ":aGVsb G8=:".chars().peekable())
        );
        assert_eq!(
            Err("byte_seq: no closing ':'"),
            Parser::parse_byte_sequence(&mut ":aGVsbG8=".chars().peekable())
        );
    }

    #[test]
    fn parse_number() {
        let mut input = ":aGVsbG8:rest".chars().peekable();
        assert_eq!(
            Err("parse_error: number does not start with digit"),
            Parser::parse_number(&mut input)
        );
        assert_eq!(":aGVsbG8:rest", input.collect::<String>());

        let mut input = "00.42 test string".chars().peekable();
        assert_eq!(
            Num::Decimal(0.42),
            Parser::parse_number(&mut input).unwrap()
        );
        assert_eq!(" test string", input.collect::<String>());

        let mut input = "-11.5555 test string".chars().peekable();
        assert_eq!(
            Err("parse_number: invalid decimal fraction length"),
            Parser::parse_number(&mut input)
        );
        assert_eq!(" test string", input.collect::<String>());

        assert_eq!(
            Num::Integer(42),
            Parser::parse_number(&mut "42".chars().peekable()).unwrap()
        );
        assert_eq!(
            Num::Integer(-42),
            Parser::parse_number(&mut "-42".chars().peekable()).unwrap()
        );
        assert_eq!(
            Num::Integer(-42),
            Parser::parse_number(&mut "-042".chars().peekable()).unwrap()
        );
        assert_eq!(
            Num::Integer(0),
            Parser::parse_number(&mut "0".chars().peekable()).unwrap()
        );
        assert_eq!(
            Num::Integer(0),
            Parser::parse_number(&mut "00".chars().peekable()).unwrap()
        );
        assert_eq!(
            Num::Integer(123456789012345),
            Parser::parse_number(&mut "123456789012345".chars().peekable()).unwrap()
        );
        assert_eq!(
            Num::Integer(-123456789012345),
            Parser::parse_number(&mut "-123456789012345".chars().peekable()).unwrap()
        );
        assert_eq!(
            Err("parse_error: number does not start with digit"),
            Parser::parse_number(&mut "- 42".chars().peekable())
        );
        assert_eq!(
            Err("parse_error: number does not start with digit"),
            Parser::parse_number(&mut "--0".chars().peekable())
        );
        assert_eq!(
            Num::Integer(2),
            Parser::parse_number(&mut "2,3".chars().peekable()).unwrap()
        );
        assert_eq!(
            Num::Integer(4),
            Parser::parse_number(&mut "4-2".chars().peekable()).unwrap()
        );
        assert_eq!(
            Err("parse_error: number does not start with digit"),
            Parser::parse_number(&mut "- 42".chars().peekable())
        );

        // for decimals
        assert_eq!(
            Num::Decimal(3.14),
            Parser::parse_number(&mut "3.14".chars().peekable()).unwrap()
        );
        assert_eq!(
            Num::Decimal(-3.14),
            Parser::parse_number(&mut "-3.14".chars().peekable()).unwrap()
        );
        assert_eq!(
            Num::Decimal(123456789012.1),
            Parser::parse_number(&mut "123456789012.1".chars().peekable()).unwrap()
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
    }
}
