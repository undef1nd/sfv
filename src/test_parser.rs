use crate::{
    integer, token_ref, BareItem, Decimal, Dictionary, Error, FromStr, InnerList, Item, List, Num,
    Parameters, ParseMore, Parser,
};
use std::error::Error as StdError;
use std::iter::FromIterator;

#[test]
fn parse() -> Result<(), Box<dyn StdError>> {
    let input = r#""some_value""#;
    let parsed_item = Item::new(BareItem::String("some_value".to_owned()));
    let expected = parsed_item;
    assert_eq!(expected, Parser::from_str(input).parse_item()?);

    let input = "12.35;a ";
    let params = Parameters::from_iter(vec![("a".to_owned(), BareItem::Boolean(true))]);
    let expected = Item::with_params(Decimal::from_str("12.35")?, params);

    assert_eq!(expected, Parser::from_str(input).parse_item()?);
    Ok(())
}

#[test]
fn parse_errors() -> Result<(), Box<dyn StdError>> {
    let input = r#""some_value¢""#;
    assert_eq!(
        Err(Error::with_index("invalid string character", 11)),
        Parser::from_str(input).parse_item()
    );
    let input = r#""some_value" trailing_text""#;
    assert_eq!(
        Err(Error::with_index(
            "trailing characters after parsed value",
            13
        )),
        Parser::from_str(input).parse_item()
    );
    assert_eq!(
        Err(Error::with_index("expected start of bare item", 0)),
        Parser::from_str("").parse_item()
    );
    Ok(())
}

#[test]
fn parse_list_of_numbers() -> Result<(), Box<dyn StdError>> {
    let input = "1,42";
    let item1 = Item::new(1);
    let item2 = Item::new(42);
    let expected_list: List = vec![item1.into(), item2.into()];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
fn parse_list_with_multiple_spaces() -> Result<(), Box<dyn StdError>> {
    let input = "1  ,  42";
    let item1 = Item::new(1);
    let item2 = Item::new(42);
    let expected_list: List = vec![item1.into(), item2.into()];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
fn parse_list_of_lists() -> Result<(), Box<dyn StdError>> {
    let input = "(1 2), (42 43)";
    let item1 = Item::new(1);
    let item2 = Item::new(2);
    let item3 = Item::new(42);
    let item4 = Item::new(43);
    let inner_list_1 = InnerList::new(vec![item1, item2]);
    let inner_list_2 = InnerList::new(vec![item3, item4]);
    let expected_list: List = vec![inner_list_1.into(), inner_list_2.into()];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
fn parse_list_empty_inner_list() -> Result<(), Box<dyn StdError>> {
    let input = "()";
    let inner_list = InnerList::new(vec![]);
    let expected_list: List = vec![inner_list.into()];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
fn parse_list_empty() -> Result<(), Box<dyn StdError>> {
    let input = "";
    let expected_list: List = vec![];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
fn parse_list_of_lists_with_param_and_spaces() -> Result<(), Box<dyn StdError>> {
    let input = "(  1  42  ); k=*";
    let item1 = Item::new(1);
    let item2 = Item::new(42);
    let inner_list_param = Parameters::from_iter(vec![(
        "k".to_owned(),
        BareItem::Token(token_ref("*").to_owned()),
    )]);
    let inner_list = InnerList::with_params(vec![item1, item2], inner_list_param);
    let expected_list: List = vec![inner_list.into()];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
fn parse_list_of_items_and_lists_with_param() -> Result<(), Box<dyn StdError>> {
    let input = r#"12, 14, (a  b); param="param_value_1", ()"#;
    let item1 = Item::new(12);
    let item2 = Item::new(14);
    let item3 = Item::new(token_ref("a").to_owned());
    let item4 = Item::new(token_ref("b").to_owned());
    let inner_list_param = Parameters::from_iter(vec![(
        "param".to_owned(),
        BareItem::String("param_value_1".to_owned()),
    )]);
    let inner_list = InnerList::with_params(vec![item3, item4], inner_list_param);
    let empty_inner_list = InnerList::new(vec![]);
    let expected_list: List = vec![
        item1.into(),
        item2.into(),
        inner_list.into(),
        empty_inner_list.into(),
    ];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
fn parse_list_errors() -> Result<(), Box<dyn StdError>> {
    let input = ",";
    assert_eq!(
        Err(Error::with_index("expected start of bare item", 0)),
        Parser::from_str(input).parse_list()
    );

    let input = "a, b c";
    assert_eq!(
        Err(Error::with_index(
            "trailing characters after list member",
            5
        )),
        Parser::from_str(input).parse_list()
    );

    let input = "a,";
    assert_eq!(
        Err(Error::with_index("trailing comma", 1)),
        Parser::from_str(input).parse_list()
    );

    let input = "a     ,    ";
    assert_eq!(
        Err(Error::with_index("trailing comma", 6)),
        Parser::from_str(input).parse_list()
    );

    let input = "a\t \t ,\t ";
    assert_eq!(
        Err(Error::with_index("trailing comma", 5)),
        Parser::from_str(input).parse_list()
    );

    let input = "a\t\t,\t\t\t";
    assert_eq!(
        Err(Error::with_index("trailing comma", 3)),
        Parser::from_str(input).parse_list()
    );

    let input = "(a b),";
    assert_eq!(
        Err(Error::with_index("trailing comma", 5)),
        Parser::from_str(input).parse_list()
    );

    let input = "(1, 2, (a b)";
    assert_eq!(
        Err(Error::with_index(
            "expected inner list delimiter (' ' or ')')",
            2
        )),
        Parser::from_str(input).parse_list()
    );

    Ok(())
}

#[test]
fn parse_inner_list_errors() -> Result<(), Box<dyn StdError>> {
    let input = "c b); a=1";
    assert_eq!(
        Err(Error::with_index("expected start of inner list", 0)),
        Parser::from_str(input).parse_inner_list()
    );

    let input = "(";
    assert_eq!(
        Err(Error::with_index("unterminated inner list", 1)),
        Parser::from_str(input).parse_inner_list()
    );
    Ok(())
}

#[test]
fn parse_inner_list_with_param_and_spaces() -> Result<(), Box<dyn StdError>> {
    let input = "(c b); a=1";
    let inner_list_param = Parameters::from_iter(vec![("a".to_owned(), 1.into())]);

    let item1 = Item::new(token_ref("c").to_owned());
    let item2 = Item::new(token_ref("b").to_owned());
    let expected = InnerList::with_params(vec![item1, item2], inner_list_param);
    assert_eq!(expected, Parser::from_str(input).parse_inner_list()?);
    Ok(())
}

#[test]
fn parse_item_int_with_space() -> Result<(), Box<dyn StdError>> {
    let input = "12 ";
    assert_eq!(Item::new(12), Parser::from_str(input).parse_item()?);
    Ok(())
}

#[test]
fn parse_item_decimal_with_bool_param_and_space() -> Result<(), Box<dyn StdError>> {
    let input = "12.35;a ";
    let param = Parameters::from_iter(vec![("a".to_owned(), BareItem::Boolean(true))]);
    assert_eq!(
        Item::with_params(Decimal::from_str("12.35")?, param),
        Parser::from_str(input).parse_item()?
    );
    Ok(())
}

#[test]
fn parse_item_number_with_param() -> Result<(), Box<dyn StdError>> {
    let param = Parameters::from_iter(vec![(
        "a1".to_owned(),
        BareItem::Token(token_ref("*").to_owned()),
    )]);
    assert_eq!(
        Item::with_params(BareItem::String("12.35".to_owned()), param),
        Parser::from_str(r#""12.35";a1=*"#).parse_item()?
    );
    Ok(())
}

#[test]
fn parse_dict_empty() -> Result<(), Box<dyn StdError>> {
    assert_eq!(Dictionary::new(), Parser::from_str("").parse_dictionary()?);
    Ok(())
}

#[test]
fn parse_dict_errors() -> Result<(), Box<dyn StdError>> {
    let input = "abc=123;a=1;b=2 def";
    assert_eq!(
        Err(Error::with_index(
            "trailing characters after dictionary member",
            16
        )),
        Parser::from_str(input).parse_dictionary()
    );
    let input = "abc=123;a=1,";
    assert_eq!(
        Err(Error::with_index("trailing comma", 11)),
        Parser::from_str(input).parse_dictionary()
    );
    Ok(())
}

#[test]
fn parse_dict_with_spaces_and_params() -> Result<(), Box<dyn StdError>> {
    let input = r#"abc=123;a=1;b=2, def=456, ghi=789;q=9;r="+w""#;
    let item1_params =
        Parameters::from_iter(vec![("a".to_owned(), 1.into()), ("b".to_owned(), 2.into())]);
    let item3_params = Parameters::from_iter(vec![
        ("q".to_owned(), 9.into()),
        ("r".to_owned(), BareItem::String("+w".to_owned())),
    ]);

    let item1 = Item::with_params(123, item1_params);
    let item2 = Item::new(456);
    let item3 = Item::with_params(789, item3_params);

    let expected_dict = Dictionary::from_iter(vec![
        ("abc".to_owned(), item1.into()),
        ("def".to_owned(), item2.into()),
        ("ghi".to_owned(), item3.into()),
    ]);
    assert_eq!(expected_dict, Parser::from_str(input).parse_dictionary()?);

    Ok(())
}

#[test]
fn parse_dict_empty_value() -> Result<(), Box<dyn StdError>> {
    let input = "a=()";
    let inner_list = InnerList::new(vec![]);
    let expected_dict = Dictionary::from_iter(vec![("a".to_owned(), inner_list.into())]);
    assert_eq!(expected_dict, Parser::from_str(input).parse_dictionary()?);
    Ok(())
}

#[test]
fn parse_dict_with_token_param() -> Result<(), Box<dyn StdError>> {
    let input = "a=1, b;foo=*, c=3";
    let item2_params = Parameters::from_iter(vec![(
        "foo".to_owned(),
        BareItem::Token(token_ref("*").to_owned()),
    )]);
    let item1 = Item::new(1);
    let item2 = Item::with_params(true, item2_params);
    let item3 = Item::new(3);
    let expected_dict = Dictionary::from_iter(vec![
        ("a".to_owned(), item1.into()),
        ("b".to_owned(), item2.into()),
        ("c".to_owned(), item3.into()),
    ]);
    assert_eq!(expected_dict, Parser::from_str(input).parse_dictionary()?);
    Ok(())
}

#[test]
fn parse_dict_multiple_spaces() -> Result<(), Box<dyn StdError>> {
    // input1, input2, input3 must be parsed into the same structure
    let item1 = Item::new(1);
    let item2 = Item::new(2);
    let expected_dict = Dictionary::from_iter(vec![
        ("a".to_owned(), item1.into()),
        ("b".to_owned(), item2.into()),
    ]);

    let input1 = "a=1 ,  b=2";
    let input2 = "a=1\t,\tb=2";
    let input3 = "a=1, b=2";
    assert_eq!(expected_dict, Parser::from_str(input1).parse_dictionary()?);
    assert_eq!(expected_dict, Parser::from_str(input2).parse_dictionary()?);
    assert_eq!(expected_dict, Parser::from_str(input3).parse_dictionary()?);

    Ok(())
}

#[test]
fn parse_bare_item() -> Result<(), Box<dyn StdError>> {
    assert_eq!(
        BareItem::Boolean(false),
        Parser::from_str("?0").parse_bare_item()?
    );
    assert_eq!(
        BareItem::String("test string".to_owned()),
        Parser::from_str(r#""test string""#).parse_bare_item()?
    );
    assert_eq!(
        BareItem::Token(token_ref("*token").to_owned()),
        Parser::from_str("*token").parse_bare_item()?
    );
    assert_eq!(
        BareItem::ByteSeq("base_64 encoding test".to_owned().into_bytes()),
        Parser::from_str(":YmFzZV82NCBlbmNvZGluZyB0ZXN0:").parse_bare_item()?
    );
    assert_eq!(
        BareItem::Decimal(Decimal::from_str("-3.55")?),
        Parser::from_str("-3.55").parse_bare_item()?
    );
    Ok(())
}

#[test]
fn parse_bare_item_errors() -> Result<(), Box<dyn StdError>> {
    assert_eq!(
        Err(Error::with_index("expected start of bare item", 0)),
        Parser::from_str("!?0").parse_bare_item()
    );
    assert_eq!(
        Err(Error::with_index("expected start of bare item", 0)),
        Parser::from_str("_11abc").parse_bare_item()
    );
    assert_eq!(
        Err(Error::with_index("expected start of bare item", 0)),
        Parser::from_str("   ").parse_bare_item()
    );
    Ok(())
}

#[test]
fn parse_bool() -> Result<(), Box<dyn StdError>> {
    let mut parser = Parser::from_str("?0gk");
    assert_eq!(false, parser.parse_bool()?);
    assert_eq!(parser.remaining(), b"gk");

    assert_eq!(false, Parser::from_str("?0").parse_bool()?);
    assert_eq!(true, Parser::from_str("?1").parse_bool()?);
    Ok(())
}

#[test]
fn parse_bool_errors() -> Result<(), Box<dyn StdError>> {
    assert_eq!(
        Err(Error::with_index("expected start of boolean ('?')", 0)),
        Parser::from_str("").parse_bool()
    );
    assert_eq!(
        Err(Error::with_index("expected boolean ('0' or '1')", 1)),
        Parser::from_str("?").parse_bool()
    );
    Ok(())
}

#[test]
fn parse_string() -> Result<(), Box<dyn StdError>> {
    let mut parser = Parser::from_str(r#""some string" ;not string"#);
    assert_eq!("some string".to_owned(), parser.parse_string()?);
    assert_eq!(parser.remaining(), " ;not string".as_bytes());

    assert_eq!(
        "test".to_owned(),
        Parser::from_str(r#""test""#).parse_string()?
    );
    assert_eq!(
        r#"te\st"#.to_owned(),
        Parser::from_str(r#""te\\st""#).parse_string()?
    );
    assert_eq!("".to_owned(), Parser::from_str(r#""""#).parse_string()?);
    assert_eq!(
        "some string".to_owned(),
        Parser::from_str(r#""some string""#).parse_string()?
    );
    Ok(())
}

#[test]
fn parse_string_errors() -> Result<(), Box<dyn StdError>> {
    assert_eq!(
        Err(Error::with_index(r#"expected start of string ('"')"#, 0)),
        Parser::from_str("test").parse_string()
    );
    assert_eq!(
        Err(Error::with_index("unterminated escape sequence", 2)),
        Parser::from_str(r#""\"#).parse_string()
    );
    assert_eq!(
        Err(Error::with_index("invalid escape sequence", 2)),
        Parser::from_str(r#""\l""#).parse_string()
    );
    assert_eq!(
        Err(Error::with_index("invalid string character", 1)),
        Parser::from_str("\"\u{1f}\"").parse_string()
    );
    assert_eq!(
        Err(Error::with_index("unterminated string", 5)),
        Parser::from_str(r#""smth"#).parse_string()
    );
    Ok(())
}

#[test]
fn parse_token() -> Result<(), Box<dyn StdError>> {
    let mut parser = Parser::from_str("*some:token}not token");
    assert_eq!(token_ref("*some:token"), parser.parse_token()?);
    assert_eq!(parser.remaining(), b"}not token");

    assert_eq!(token_ref("token"), Parser::from_str("token").parse_token()?);
    assert_eq!(
        token_ref("a_b-c.d3:f%00/*"),
        Parser::from_str("a_b-c.d3:f%00/*").parse_token()?
    );
    assert_eq!(
        token_ref("TestToken"),
        Parser::from_str("TestToken").parse_token()?
    );
    assert_eq!(
        token_ref("some"),
        Parser::from_str("some@token").parse_token()?
    );
    assert_eq!(
        token_ref("*TestToken*"),
        Parser::from_str("*TestToken*").parse_token()?
    );
    assert_eq!(token_ref("*"), Parser::from_str("*[@:token").parse_token()?);
    assert_eq!(
        token_ref("test"),
        Parser::from_str("test token").parse_token()?
    );

    Ok(())
}

#[test]
fn parse_token_errors() -> Result<(), Box<dyn StdError>> {
    let mut parser = Parser::from_str("765token");
    assert_eq!(
        Err(Error::with_index("expected start of token", 0)),
        parser.parse_token()
    );
    assert_eq!(parser.remaining(), b"765token");

    assert_eq!(
        Err(Error::with_index("expected start of token", 0)),
        Parser::from_str("7token").parse_token()
    );
    assert_eq!(
        Err(Error::with_index("expected start of token", 0)),
        Parser::from_str("").parse_token()
    );
    Ok(())
}

#[test]
fn parse_byte_sequence() -> Result<(), Box<dyn StdError>> {
    let mut parser = Parser::from_str(":aGVsbG8:rest_of_str");
    assert_eq!(
        "hello".to_owned().into_bytes(),
        parser.parse_byte_sequence()?
    );
    assert_eq!(parser.remaining(), b"rest_of_str");

    assert_eq!(
        "hello".to_owned().into_bytes(),
        Parser::from_str(":aGVsbG8:").parse_byte_sequence()?
    );
    assert_eq!(
        "test_encode".to_owned().into_bytes(),
        Parser::from_str(":dGVzdF9lbmNvZGU:").parse_byte_sequence()?
    );
    assert_eq!(
        "new:year tree".to_owned().into_bytes(),
        Parser::from_str(":bmV3OnllYXIgdHJlZQ==:").parse_byte_sequence()?
    );
    assert_eq!(
        "".to_owned().into_bytes(),
        Parser::from_str("::").parse_byte_sequence()?
    );
    Ok(())
}

#[test]
fn parse_byte_sequence_errors() -> Result<(), Box<dyn StdError>> {
    assert_eq!(
        Err(Error::with_index(
            "expected start of byte sequence (':')",
            0
        )),
        Parser::from_str("aGVsbG8").parse_byte_sequence()
    );
    assert_eq!(
        Err(Error::with_index("invalid byte sequence", 6)),
        Parser::from_str(":aGVsb G8=:").parse_byte_sequence()
    );
    assert_eq!(
        Err(Error::with_index("unterminated byte sequence", 9)),
        Parser::from_str(":aGVsbG8=").parse_byte_sequence()
    );
    Ok(())
}

#[test]
fn parse_number_int() -> Result<(), Box<dyn StdError>> {
    let mut parser = Parser::from_str("-733333333332d.14");
    assert_eq!(Num::Integer(integer(-733333333332)), parser.parse_number()?);
    assert_eq!(parser.remaining(), b"d.14");

    assert_eq!(
        Num::Integer(integer(42)),
        Parser::from_str("42").parse_number()?
    );
    assert_eq!(
        Num::Integer(integer(-42)),
        Parser::from_str("-42").parse_number()?
    );
    assert_eq!(
        Num::Integer(integer(-42)),
        Parser::from_str("-042").parse_number()?
    );
    assert_eq!(
        Num::Integer(integer(0)),
        Parser::from_str("0").parse_number()?
    );
    assert_eq!(
        Num::Integer(integer(0)),
        Parser::from_str("00").parse_number()?
    );
    assert_eq!(
        Num::Integer(integer(123456789012345)),
        Parser::from_str("123456789012345").parse_number()?
    );
    assert_eq!(
        Num::Integer(integer(-123456789012345)),
        Parser::from_str("-123456789012345").parse_number()?
    );
    assert_eq!(
        Num::Integer(integer(2)),
        Parser::from_str("2,3").parse_number()?
    );
    assert_eq!(
        Num::Integer(integer(4)),
        Parser::from_str("4-2").parse_number()?
    );
    assert_eq!(
        Num::Integer(integer(-999999999999999)),
        Parser::from_str("-999999999999999").parse_number()?
    );
    assert_eq!(
        Num::Integer(integer(999999999999999)),
        Parser::from_str("999999999999999").parse_number()?
    );

    Ok(())
}

#[test]
fn parse_number_decimal() -> Result<(), Box<dyn StdError>> {
    let mut parser = Parser::from_str("00.42 test string");
    assert_eq!(
        Num::Decimal(Decimal::from_str("0.42")?),
        parser.parse_number()?
    );
    assert_eq!(parser.remaining(), b" test string");

    assert_eq!(
        Num::Decimal(Decimal::from_str("1.5")?),
        Parser::from_str("1.5.4.").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::from_str("1.8")?),
        Parser::from_str("1.8.").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::from_str("1.7")?),
        Parser::from_str("1.7.0").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::from_str("3.14")?),
        Parser::from_str("3.14").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::from_str("-3.14")?),
        Parser::from_str("-3.14").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::from_str("123456789012.1")?),
        Parser::from_str("123456789012.1").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::from_str("1234567890.112")?),
        Parser::from_str("1234567890.112").parse_number()?
    );

    Ok(())
}

#[test]
fn parse_number_errors() -> Result<(), Box<dyn StdError>> {
    let mut parser = Parser::from_str(":aGVsbG8:rest");
    assert_eq!(
        Err(Error::with_index("expected digit", 0)),
        parser.parse_number()
    );
    assert_eq!(parser.remaining(), b":aGVsbG8:rest");

    let mut parser = Parser::from_str("-11.5555 test string");
    assert_eq!(
        Err(Error::with_index("too many digits after decimal point", 7)),
        parser.parse_number()
    );
    assert_eq!(parser.remaining(), b"5 test string");

    assert_eq!(
        Err(Error::with_index("expected digit", 1)),
        Parser::from_str("--0").parse_number()
    );
    assert_eq!(
        Err(Error::with_index(
            "too many digits before decimal point",
            13
        )),
        Parser::from_str("1999999999999.1").parse_number()
    );
    assert_eq!(
        Err(Error::with_index("trailing decimal point", 11)),
        Parser::from_str("19888899999.").parse_number()
    );
    assert_eq!(
        Err(Error::with_index("too many digits", 15)),
        Parser::from_str("1999999999999999").parse_number()
    );
    assert_eq!(
        Err(Error::with_index("too many digits after decimal point", 15)),
        Parser::from_str("19999999999.99991").parse_number()
    );
    assert_eq!(
        Err(Error::with_index("expected digit", 1)),
        Parser::from_str("- 42").parse_number()
    );
    assert_eq!(
        Err(Error::with_index("trailing decimal point", 1)),
        Parser::from_str("1..4").parse_number()
    );
    assert_eq!(
        Err(Error::with_index("expected digit", 1)),
        Parser::from_str("-").parse_number()
    );
    assert_eq!(
        Err(Error::with_index("trailing decimal point", 2)),
        Parser::from_str("-5. 14").parse_number()
    );
    assert_eq!(
        Err(Error::with_index("trailing decimal point", 1)),
        Parser::from_str("7. 1").parse_number()
    );
    assert_eq!(
        Err(Error::with_index("too many digits after decimal point", 6)),
        Parser::from_str("-7.3333333333").parse_number()
    );
    assert_eq!(
        Err(Error::with_index(
            "too many digits before decimal point",
            14
        )),
        Parser::from_str("-7333333333323.12").parse_number()
    );

    Ok(())
}

#[test]
fn parse_params_string() -> Result<(), Box<dyn StdError>> {
    let input = r#";b="param_val""#;
    let expected = Parameters::from_iter(vec![(
        "b".to_owned(),
        BareItem::String("param_val".to_owned()),
    )]);
    assert_eq!(expected, Parser::from_str(input).parse_parameters()?);
    Ok(())
}

#[test]
fn parse_params_bool() -> Result<(), Box<dyn StdError>> {
    let input = ";b;a";
    let expected = Parameters::from_iter(vec![
        ("b".to_owned(), BareItem::Boolean(true)),
        ("a".to_owned(), BareItem::Boolean(true)),
    ]);
    assert_eq!(expected, Parser::from_str(input).parse_parameters()?);
    Ok(())
}

#[test]
fn parse_params_mixed_types() -> Result<(), Box<dyn StdError>> {
    let input = ";key1=?0;key2=746.15";
    let expected = Parameters::from_iter(vec![
        ("key1".to_owned(), BareItem::Boolean(false)),
        ("key2".to_owned(), Decimal::from_str("746.15")?.into()),
    ]);
    assert_eq!(expected, Parser::from_str(input).parse_parameters()?);
    Ok(())
}

#[test]
fn parse_params_with_spaces() -> Result<(), Box<dyn StdError>> {
    let input = "; key1=?0; key2=11111";
    let expected = Parameters::from_iter(vec![
        ("key1".to_owned(), BareItem::Boolean(false)),
        ("key2".to_owned(), 11111.into()),
    ]);
    assert_eq!(expected, Parser::from_str(input).parse_parameters()?);
    Ok(())
}

#[test]
fn parse_params_empty() -> Result<(), Box<dyn StdError>> {
    assert_eq!(
        Parameters::new(),
        Parser::from_str(" key1=?0; key2=11111").parse_parameters()?
    );
    assert_eq!(Parameters::new(), Parser::from_str("").parse_parameters()?);
    assert_eq!(
        Parameters::new(),
        Parser::from_str("[;a=1").parse_parameters()?
    );
    assert_eq!(Parameters::new(), Parser::from_str("").parse_parameters()?);
    Ok(())
}

#[test]
fn parse_key() -> Result<(), Box<dyn StdError>> {
    assert_eq!("a".to_owned(), Parser::from_str("a=1").parse_key()?);
    assert_eq!("a1".to_owned(), Parser::from_str("a1=10").parse_key()?);
    assert_eq!("*1".to_owned(), Parser::from_str("*1=10").parse_key()?);
    assert_eq!("f".to_owned(), Parser::from_str("f[f=10").parse_key()?);
    Ok(())
}

#[test]
fn parse_key_errors() -> Result<(), Box<dyn StdError>> {
    assert_eq!(
        Err(Error::with_index(
            "expected start of key ('a'-'z' or '*')",
            0
        )),
        Parser::from_str("[*f=10").parse_key()
    );
    Ok(())
}

#[test]
fn parse_more_list() -> Result<(), Box<dyn StdError>> {
    let item1 = Item::new(1);
    let item2 = Item::new(2);
    let item3 = Item::new(42);
    let inner_list_1 = InnerList::new(vec![item1, item2]);
    let expected_list: List = vec![inner_list_1.into(), item3.into()];

    let mut parsed_header = Parser::from_str("(1 2)").parse_list()?;
    let _ = parsed_header.parse_more("42".as_bytes())?;
    assert_eq!(expected_list, parsed_header);
    Ok(())
}

#[test]
fn parse_more_dict() -> Result<(), Box<dyn StdError>> {
    let item2_params = Parameters::from_iter(vec![(
        "foo".to_owned(),
        BareItem::Token(token_ref("*").to_owned()),
    )]);
    let item1 = Item::new(1);
    let item2 = Item::with_params(true, item2_params);
    let item3 = Item::new(3);
    let expected_dict = Dictionary::from_iter(vec![
        ("a".to_owned(), item1.into()),
        ("b".to_owned(), item2.into()),
        ("c".to_owned(), item3.into()),
    ]);

    let mut parsed_header = Parser::from_str("a=1, b;foo=*\t\t").parse_dictionary()?;
    let _ = parsed_header.parse_more(" c=3".as_bytes())?;
    assert_eq!(expected_dict, parsed_header);
    Ok(())
}

#[test]
fn parse_more_errors() -> Result<(), Box<dyn StdError>> {
    let parsed_dict_header = Parser::from_str("a=1, b;foo=*")
        .parse_dictionary()?
        .parse_more(",a".as_bytes());
    assert!(parsed_dict_header.is_err());

    let parsed_list_header = Parser::from_str("a, b;foo=*")
        .parse_list()?
        .parse_more("(a, 2)".as_bytes());
    assert!(parsed_list_header.is_err());
    Ok(())
}
