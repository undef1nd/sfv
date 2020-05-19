#[allow(dead_code)]
#[derive(Debug, PartialEq)]
enum Item {
    Decimal(i64),
    Integer(i64),
    String(String),
    ByteSeq(Vec<u8>),
    Boolean(bool),
    Token(String),
}

#[derive(Debug)]
struct Parser {
    input: String,
}

impl Parser {
    fn parse_bare_item(input: &str) -> Result<Item, ()> {
        if input.starts_with('?') {
            Ok(Item::Boolean(Parser::parse_bool(input)?.0))
        } else {
            return Err(());
        }
    }

    fn parse_bool(input: &str) -> Result<(bool, &str), ()> {
        let mut iter = input.chars();

        if iter.next() != Some('?') {
            return Err(());
        }

        match iter.next() {
            Some('0') => Ok((false, &input["?0".len()..])),
            Some('1') => Ok((true, &input["?0".len()..])),
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

    #[test]
    fn parse_bare_item() {
        assert_eq!(Item::Boolean(false), Parser::parse_bare_item("?0").unwrap());
    }

    #[test]
    fn parse_bool() {
        assert_eq!((false, ""), Parser::parse_bool("?0").unwrap());
        assert_eq!((true, ""), Parser::parse_bool("?1").unwrap());
        assert_eq!((false, "gk"), Parser::parse_bool("?0gk").unwrap());
        assert_eq!(Err(()), Parser::parse_bool(""));
        assert_eq!(Err(()), Parser::parse_bool("?"));
    }
}
