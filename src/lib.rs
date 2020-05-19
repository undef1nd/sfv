#[allow(dead_code)]
use indexmap::IndexMap;
use std::iter::Peekable;
use std::path::Iter;
use std::str::Chars;

type Parameters = IndexMap<String, BareItem>;

#[derive(Debug)]
struct Item {
    bare_item: BareItem,
    parameters: Option<Parameters>,
}

#[derive(Debug)]
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
    fn new(input_string: String) -> Self {
        Parser {
            input_str: input_string,
        }
    }

    fn parse_item(self) -> Result<Item, ()> {
        // parse item
        // parse parameters
        // return Item { ... }
        let iter = self.input_str.chars().peekable();
        let bare_item = Self::parse_bare_item(iter)?;
        Ok(Item {
            bare_item,
            parameters: None,
        })
    }

    fn parse_bare_item(mut input: Peekable<Chars>) -> Result<BareItem, ()> {
        match input.peek() {
            Some(&'?') => Ok(BareItem::Boolean(Self::parse_bool(input)?.0)),
            _ => Err(()),
        }
    }

    fn parse_bool(mut input: Peekable<Chars>) -> Result<(bool, Peekable<Chars>), ()> {
        if input.next() != Some('?') {
            return Err(());
        }

        match input.next() {
            Some('0') => Ok((false, input)),
            Some('1') => Ok((true, input)),
            _ => Err(()),
        }
    }

    fn parse_parameters() -> Result<(), ()> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn parse_bare_item() {
    //     assert_eq!(BareItem::Boolean(false), Parser::parse_bare_item("?0").unwrap());
    // }
    //
    // #[test]
    // fn parse_bool() {
    //     assert_eq!((false, ""), Parser::parse_bool("?0").unwrap());
    //     assert_eq!((true, ""), Parser::parse_bool("?1").unwrap());
    //     assert_eq!((false, "gk"), Parser::parse_bool("?0gk").unwrap());
    //     assert_eq!(Err(()), Parser::parse_bool(""));
    //     assert_eq!(Err(()), Parser::parse_bool("?"));
    // }
}
