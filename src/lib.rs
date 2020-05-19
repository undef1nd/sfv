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
        let iter = self.input_str.chars();
        let bare_item = Self::parse_bare_item(iter)?;
        Ok(Item {
            bare_item,
            parameters: None,
        })
    }

    fn parse_bare_item(mut input: Chars) -> Result<BareItem, ()> {
        match input.clone().peekable().peek() {
            Some(&'?') => Ok(BareItem::Boolean(Self::parse_bool(input)?.0)),
            _ => Err(()),
        }
    }

    fn parse_bool(mut input: Chars) -> Result<(bool, Chars), ()> {
        if input.next() != Some('?') {
            return Err(());
        }

        match input.next() {
            Some('0') => Ok((false, input)),
            Some('1') => Ok((true, input)),
            _ => Err(()),
        }
    }

    // fn parse_parameters() -> Result<(), ()> {
    //     unimplemented!()
    // }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // #[test]
    // fn parse_bare_item() {
    //     assert_eq!(BareItem::Boolean(false), Parser::parse_bare_item("?0").unwrap());
    // }
    //
    #[test]
    fn parse_bool() {
        let mut test_data = HashMap::new();
        test_data.insert("?0", (false, ""));
        test_data.insert("?1", (true, ""));
        test_data.insert("?1gl", (true, "gl"));


        for (input, expected_output) in test_data {
            let actual_output = Parser::parse_bool(input.chars());
            let (actual_bool, actual_iter) = actual_output.unwrap();
            let actual_iter = actual_iter.as_str();
            assert_eq!(expected_output, (actual_bool, actual_iter));
        }
    }

    #[test]
    #[should_panic]
    fn parse_bool_invalid() {
        Parser::parse_bool("?".chars()).unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_bool_empty_str() {
        Parser::parse_bool("".chars()).unwrap();
    }
}
