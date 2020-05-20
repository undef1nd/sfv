#[allow(dead_code)]
use indexmap::IndexMap;
use std::fs::read;
use std::iter::Peekable;
use std::path::Iter;
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
    //     // parse item
    //     // parse parameters
    //     // return Item { ... }
    //     let iter = self.input_str.chars().peekable();
    //     let bare_item = Self::parse_bare_item(iter)?;
    //     Ok(Item {
    //         bare_item,
    //         parameters: None,
    //     })
    // }

    fn parse_bare_item(mut input: &mut Peekable<Chars>) -> Result<BareItem, ()> {
        match input.peek() {
            Some(&'?') => Ok(BareItem::Boolean(Self::parse_bool(&mut input)?)),
            Some(&'"') => Ok(BareItem::String(Self::parse_string(&mut input)?)),
            _ => Err(()),
        }
    }

    fn parse_bool(input: &mut Peekable<Chars>) -> Result<bool, ()> {
        if input.next() != Some('?') {
            return Err(());
        }

        match input.next() {
            Some('0') => Ok(false),
            Some('1') => Ok(true),
            _ => Err(()),
        }
    }

    fn parse_string(input: &mut Peekable<Chars>) -> Result<String, ()> {
        if input.next() != Some('\"') {
            return Err(());
        }

        let mut output_string = String::from("");
        while let Some(curr_char) = input.next() {
            if curr_char == '\\' {
                match input.next() {
                    Some('\\') =>  { output_string.push(curr_char) },
                    Some ('\"') => { output_string.push(curr_char) },
                    None => { return Err(()) }
                    _ => { return Err(()) }
                }
            } else if (curr_char >= '\x00' && curr_char <= '\x1f') || curr_char == '\x7f' {
                return Err(());
            } else if curr_char == '\"' {
                return Ok(output_string);
            } else {
                output_string.push(curr_char);
            }
        }
        Err(())
    }

    fn parse_parameters() -> Result<(), ()> {
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
    }
    #[test]
    fn parse_bool() {
        assert_eq!(false, Parser::parse_bool(&mut "?0".chars().peekable()).unwrap());
        assert_eq!(true, Parser::parse_bool(&mut "?1".chars().peekable()).unwrap());
        assert_eq!(false, Parser::parse_bool(&mut "?0gk".chars().peekable()).unwrap());
        assert_eq!(Err(()), Parser::parse_bool(&mut "".chars().peekable()));
        assert_eq!(Err(()), Parser::parse_bool(&mut "?".chars().peekable()));
    }

    #[test]
    fn parse_string() {
        assert_eq!(
            "test".to_owned(),
            Parser::parse_string(&mut "\"test\"".chars().peekable()).unwrap()
        );
        assert_eq!(
            "".to_owned(),
            Parser::parse_string(&mut "\"\"".chars().peekable()).unwrap()
        );
        assert_eq!(Err(()), Parser::parse_string(&mut "test".chars().peekable()));
        assert_eq!(Err(()), Parser::parse_string(&mut "\"\\".chars().peekable()));
        assert_eq!(Err(()), Parser::parse_string(&mut "\"\\l\"".chars().peekable()));
        assert_eq!(Err(()), Parser::parse_string(&mut "\"\u{1f}\"".chars().peekable()));
        assert_eq!(Err(()), Parser::parse_string(&mut "\"smth".chars().peekable()));
    }
}
