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
enum BareItem {
    Decimal(i64),
    Integer(i64),
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
            // Some(&'*') => Ok(BareItem::Token(Self::parse_token(&mut input)?)),
            Some(&c) if c == '*' || c.is_ascii_alphabetic() => {
                Ok(BareItem::Token(Self::parse_token(&mut input)?))
            }
            _ => Err("parse_bare_item: not an item"),
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
        //https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#parse-token

        if let Some(first_char) = input.peek() {
            if !first_char.is_ascii_alphabetic() && first_char != &'*' {
                return Err("token: first char is not ALPHA or '*'");
            }
        } else {
            return Err("token: empty input string");
        }
        let mut output_string = String::from("");
        while let Some(curr_char) = input.peek() {
            if !utils::is_tchar(curr_char) && curr_char != &':' && curr_char != &'/' {
                return Ok(output_string);
            }

            match input.next() {
                Some(c) => output_string.push(c),
                None => return Err("token: end of the string"),
            }
        }
        Ok(output_string)
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

}
