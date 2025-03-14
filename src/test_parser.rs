use crate::visitor::Ignored;
use crate::{integer, key_ref, string_ref, token_ref, Decimal, Error, Num, Parser, RefBareItem};
use std::convert::TryFrom;

#[cfg(feature = "parsed-types")]
use crate::{BareItem, Dictionary, InnerList, Item, List, Parameters};

#[cfg(feature = "parsed-types")]
use std::iter::FromIterator;

#[test]
#[cfg(feature = "parsed-types")]
fn parse() -> Result<(), Error> {
    let input = r#""some_value""#;
    let parsed_item = Item::new(string_ref("some_value"));
    let expected = parsed_item;
    assert_eq!(expected, Parser::from_str(input).parse_item()?);

    let input = "12.35;a ";
    let params = Parameters::from_iter(vec![(key_ref("a").to_owned(), BareItem::Boolean(true))]);
    let expected = Item::with_params(Decimal::try_from(12.35)?, params);

    assert_eq!(expected, Parser::from_str(input).parse_item()?);
    Ok(())
}

#[test]
fn parse_errors() {
    let input = r#""some_value¢""#;
    assert_eq!(
        Err(Error::with_index("invalid string character", 11)),
        Parser::from_str(input).parse_item_with_visitor(Ignored)
    );
    let input = r#""some_value" trailing_text""#;
    assert_eq!(
        Err(Error::with_index(
            "trailing characters after parsed value",
            13
        )),
        Parser::from_str(input).parse_item_with_visitor(Ignored)
    );
    assert_eq!(
        Err(Error::with_index("expected start of bare item", 0)),
        Parser::from_str("").parse_item_with_visitor(Ignored)
    );
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_list_of_numbers() -> Result<(), Error> {
    let input = "1,42";
    let item1 = Item::new(1);
    let item2 = Item::new(42);
    let expected_list: List = vec![item1.into(), item2.into()];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_list_with_multiple_spaces() -> Result<(), Error> {
    let input = "1  ,  42";
    let item1 = Item::new(1);
    let item2 = Item::new(42);
    let expected_list: List = vec![item1.into(), item2.into()];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_list_of_lists() -> Result<(), Error> {
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
#[cfg(feature = "parsed-types")]
fn parse_list_empty_inner_list() -> Result<(), Error> {
    let input = "()";
    let inner_list = InnerList::new(vec![]);
    let expected_list: List = vec![inner_list.into()];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_list_empty() -> Result<(), Error> {
    let input = "";
    let expected_list: List = vec![];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_list_of_lists_with_param_and_spaces() -> Result<(), Error> {
    let input = "(  1  42  ); k=*";
    let item1 = Item::new(1);
    let item2 = Item::new(42);
    let inner_list_param = Parameters::from_iter(vec![(
        key_ref("k").to_owned(),
        BareItem::Token(token_ref("*").to_owned()),
    )]);
    let inner_list = InnerList::with_params(vec![item1, item2], inner_list_param);
    let expected_list: List = vec![inner_list.into()];
    assert_eq!(expected_list, Parser::from_str(input).parse_list()?);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_list_of_items_and_lists_with_param() -> Result<(), Error> {
    let input = r#"12, 14, (a  b); param="param_value_1", ()"#;
    let item1 = Item::new(12);
    let item2 = Item::new(14);
    let item3 = Item::new(token_ref("a"));
    let item4 = Item::new(token_ref("b"));
    let inner_list_param = Parameters::from_iter(vec![(
        key_ref("param").to_owned(),
        BareItem::String(string_ref("param_value_1").to_owned()),
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
fn parse_list_errors() {
    let input = ",";
    assert_eq!(
        Err(Error::with_index("expected start of bare item", 0)),
        Parser::from_str(input).parse_list_with_visitor(&mut Ignored)
    );

    let input = "a, b c";
    assert_eq!(
        Err(Error::with_index(
            "trailing characters after list member",
            5
        )),
        Parser::from_str(input).parse_list_with_visitor(&mut Ignored)
    );

    let input = "a,";
    assert_eq!(
        Err(Error::with_index("trailing comma", 1)),
        Parser::from_str(input).parse_list_with_visitor(&mut Ignored)
    );

    let input = "a     ,    ";
    assert_eq!(
        Err(Error::with_index("trailing comma", 6)),
        Parser::from_str(input).parse_list_with_visitor(&mut Ignored)
    );

    let input = "a\t \t ,\t ";
    assert_eq!(
        Err(Error::with_index("trailing comma", 5)),
        Parser::from_str(input).parse_list_with_visitor(&mut Ignored)
    );

    let input = "a\t\t,\t\t\t";
    assert_eq!(
        Err(Error::with_index("trailing comma", 3)),
        Parser::from_str(input).parse_list_with_visitor(&mut Ignored)
    );

    let input = "(a b),";
    assert_eq!(
        Err(Error::with_index("trailing comma", 5)),
        Parser::from_str(input).parse_list_with_visitor(&mut Ignored)
    );

    let input = "(1, 2, (a b)";
    assert_eq!(
        Err(Error::with_index(
            "expected inner list delimiter (' ' or ')')",
            2
        )),
        Parser::from_str(input).parse_list_with_visitor(&mut Ignored)
    );
}

#[test]
fn parse_inner_list_errors() {
    let input = "c b); a=1";
    assert_eq!(
        Err(Error::with_index("expected start of inner list", 0)),
        Parser::from_str(input).parse_inner_list(Ignored)
    );

    let input = "(";
    assert_eq!(
        Err(Error::with_index("unterminated inner list", 1)),
        Parser::from_str(input).parse_inner_list(Ignored)
    );
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_inner_list_with_param_and_spaces() -> Result<(), Error> {
    let input = "(c b); a=1";
    let inner_list_param = Parameters::from_iter(vec![(key_ref("a").to_owned(), 1.into())]);

    let item1 = Item::new(token_ref("c"));
    let item2 = Item::new(token_ref("b"));
    let expected = InnerList::with_params(vec![item1, item2], inner_list_param);
    let mut inner_list = InnerList::default();
    Parser::from_str(input).parse_inner_list(&mut inner_list)?;
    assert_eq!(expected, inner_list);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_item_int_with_space() -> Result<(), Error> {
    let input = "12 ";
    assert_eq!(Item::new(12), Parser::from_str(input).parse_item()?);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_item_decimal_with_bool_param_and_space() -> Result<(), Error> {
    let input = "12.35;a ";
    let param = Parameters::from_iter(vec![(key_ref("a").to_owned(), BareItem::Boolean(true))]);
    assert_eq!(
        Item::with_params(Decimal::try_from(12.35)?, param),
        Parser::from_str(input).parse_item()?
    );
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_item_number_with_param() -> Result<(), Error> {
    let param = Parameters::from_iter(vec![(
        key_ref("a1").to_owned(),
        BareItem::Token(token_ref("*").to_owned()),
    )]);
    assert_eq!(
        Item::with_params(string_ref("12.35"), param),
        Parser::from_str(r#""12.35";a1=*"#).parse_item()?
    );
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_dict_empty() -> Result<(), Error> {
    assert_eq!(Dictionary::new(), Parser::from_str("").parse_dictionary()?);
    Ok(())
}

#[test]
fn parse_dict_errors() {
    let input = "abc=123;a=1;b=2 def";
    assert_eq!(
        Err(Error::with_index(
            "trailing characters after dictionary member",
            16
        )),
        Parser::from_str(input).parse_dictionary_with_visitor(&mut Ignored)
    );
    let input = "abc=123;a=1,";
    assert_eq!(
        Err(Error::with_index("trailing comma", 11)),
        Parser::from_str(input).parse_dictionary_with_visitor(&mut Ignored)
    );
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_dict_with_spaces_and_params() -> Result<(), Error> {
    let input = r#"abc=123;a=1;b=2, def=456, ghi=789;q=9;r="+w""#;
    let item1_params = Parameters::from_iter(vec![
        (key_ref("a").to_owned(), 1.into()),
        (key_ref("b").to_owned(), 2.into()),
    ]);
    let item3_params = Parameters::from_iter(vec![
        (key_ref("q").to_owned(), 9.into()),
        (
            key_ref("r").to_owned(),
            BareItem::String(string_ref("+w").to_owned()),
        ),
    ]);

    let item1 = Item::with_params(123, item1_params);
    let item2 = Item::new(456);
    let item3 = Item::with_params(789, item3_params);

    let expected_dict = Dictionary::from_iter(vec![
        (key_ref("abc").to_owned(), item1.into()),
        (key_ref("def").to_owned(), item2.into()),
        (key_ref("ghi").to_owned(), item3.into()),
    ]);
    assert_eq!(expected_dict, Parser::from_str(input).parse_dictionary()?);

    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_dict_empty_value() -> Result<(), Error> {
    let input = "a=()";
    let inner_list = InnerList::new(vec![]);
    let expected_dict = Dictionary::from_iter(vec![(key_ref("a").to_owned(), inner_list.into())]);
    assert_eq!(expected_dict, Parser::from_str(input).parse_dictionary()?);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_dict_with_token_param() -> Result<(), Error> {
    let input = "a=1, b;foo=*, c=3";
    let item2_params = Parameters::from_iter(vec![(
        key_ref("foo").to_owned(),
        BareItem::Token(token_ref("*").to_owned()),
    )]);
    let item1 = Item::new(1);
    let item2 = Item::with_params(true, item2_params);
    let item3 = Item::new(3);
    let expected_dict = Dictionary::from_iter(vec![
        (key_ref("a").to_owned(), item1.into()),
        (key_ref("b").to_owned(), item2.into()),
        (key_ref("c").to_owned(), item3.into()),
    ]);
    assert_eq!(expected_dict, Parser::from_str(input).parse_dictionary()?);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_dict_multiple_spaces() -> Result<(), Error> {
    // input1, input2, input3 must be parsed into the same structure
    let item1 = Item::new(1);
    let item2 = Item::new(2);
    let expected_dict = Dictionary::from_iter(vec![
        (key_ref("a").to_owned(), item1.into()),
        (key_ref("b").to_owned(), item2.into()),
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
fn parse_bare_item() -> Result<(), Error> {
    assert_eq!(
        RefBareItem::Boolean(false),
        Parser::from_str("?0").parse_bare_item()?
    );
    assert_eq!(
        RefBareItem::String(string_ref("test string")),
        Parser::from_str(r#""test string""#).parse_bare_item()?
    );
    assert_eq!(
        RefBareItem::Token(token_ref("*token")),
        Parser::from_str("*token").parse_bare_item()?
    );
    assert_eq!(
        RefBareItem::ByteSeq(b"base_64 encoding test"),
        Parser::from_str(":YmFzZV82NCBlbmNvZGluZyB0ZXN0:").parse_bare_item()?
    );
    assert_eq!(
        RefBareItem::Decimal(Decimal::try_from(-3.55)?),
        Parser::from_str("-3.55").parse_bare_item()?
    );
    Ok(())
}

#[test]
fn parse_bare_item_errors() {
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
}

#[test]
fn parse_bool() -> Result<(), Error> {
    let mut parser = Parser::from_str("?0gk");
    assert_eq!(false, parser.parse_bool()?);
    assert_eq!(parser.remaining(), b"gk");

    assert_eq!(false, Parser::from_str("?0").parse_bool()?);
    assert_eq!(true, Parser::from_str("?1").parse_bool()?);
    Ok(())
}

#[test]
fn parse_bool_errors() {
    assert_eq!(
        Err(Error::with_index("expected start of boolean ('?')", 0)),
        Parser::from_str("").parse_bool()
    );
    assert_eq!(
        Err(Error::with_index("expected boolean ('0' or '1')", 1)),
        Parser::from_str("?").parse_bool()
    );
}

#[test]
fn parse_string() -> Result<(), Error> {
    let mut parser = Parser::from_str(r#""some string" ;not string"#);
    assert_eq!(string_ref("some string"), parser.parse_string()?);
    assert_eq!(parser.remaining(), " ;not string".as_bytes());

    assert_eq!(
        string_ref("test"),
        Parser::from_str(r#""test""#).parse_string()?
    );
    assert_eq!(
        string_ref(r#"te\st"#),
        Parser::from_str(r#""te\\st""#).parse_string()?
    );
    assert_eq!(string_ref(""), Parser::from_str(r#""""#).parse_string()?);
    assert_eq!(
        string_ref("some string"),
        Parser::from_str(r#""some string""#).parse_string()?
    );
    Ok(())
}

#[test]
fn parse_string_errors() {
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
}

#[test]
fn parse_token() -> Result<(), Error> {
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
fn parse_token_errors() {
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
}

#[test]
fn parse_byte_sequence() -> Result<(), Error> {
    let mut parser = Parser::from_str(":aGVsbG8:rest_of_str");
    assert_eq!("hello".as_bytes(), parser.parse_byte_sequence()?);
    assert_eq!(parser.remaining(), b"rest_of_str");

    assert_eq!(
        "hello".as_bytes(),
        Parser::from_str(":aGVsbG8:").parse_byte_sequence()?
    );
    assert_eq!(
        "test_encode".as_bytes(),
        Parser::from_str(":dGVzdF9lbmNvZGU:").parse_byte_sequence()?
    );
    assert_eq!(
        "new:year tree".as_bytes(),
        Parser::from_str(":bmV3OnllYXIgdHJlZQ==:").parse_byte_sequence()?
    );
    assert_eq!("".as_bytes(), Parser::from_str("::").parse_byte_sequence()?);
    Ok(())
}

#[test]
fn parse_byte_sequence_errors() {
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
}

#[test]
fn parse_number_int() -> Result<(), Error> {
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
fn parse_number_decimal() -> Result<(), Error> {
    let mut parser = Parser::from_str("00.42 test string");
    assert_eq!(
        Num::Decimal(Decimal::try_from(0.42)?),
        parser.parse_number()?
    );
    assert_eq!(parser.remaining(), b" test string");

    assert_eq!(
        Num::Decimal(Decimal::try_from(1.5)?),
        Parser::from_str("1.5.4.").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::try_from(1.8)?),
        Parser::from_str("1.8.").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::try_from(1.7)?),
        Parser::from_str("1.7.0").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::try_from(3.14)?),
        Parser::from_str("3.14").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::try_from(-3.14)?),
        Parser::from_str("-3.14").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::try_from(123456789012.1)?),
        Parser::from_str("123456789012.1").parse_number()?
    );
    assert_eq!(
        Num::Decimal(Decimal::try_from(1234567890.112)?),
        Parser::from_str("1234567890.112").parse_number()?
    );

    Ok(())
}

#[test]
fn parse_number_errors() {
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
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_params_string() -> Result<(), Error> {
    let input = r#";b="param_val""#;
    let expected = Parameters::from_iter(vec![(
        key_ref("b").to_owned(),
        BareItem::String(string_ref("param_val").to_owned()),
    )]);
    let mut params = Parameters::new();
    Parser::from_str(input).parse_parameters(&mut params)?;
    assert_eq!(expected, params);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_params_bool() -> Result<(), Error> {
    let input = ";b;a";
    let expected = Parameters::from_iter(vec![
        (key_ref("b").to_owned(), BareItem::Boolean(true)),
        (key_ref("a").to_owned(), BareItem::Boolean(true)),
    ]);
    let mut params = Parameters::new();
    Parser::from_str(input).parse_parameters(&mut params)?;
    assert_eq!(expected, params);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_params_mixed_types() -> Result<(), Error> {
    let input = ";key1=?0;key2=746.15";
    let expected = Parameters::from_iter(vec![
        (key_ref("key1").to_owned(), BareItem::Boolean(false)),
        (
            key_ref("key2").to_owned(),
            Decimal::try_from(746.15)?.into(),
        ),
    ]);
    let mut params = Parameters::new();
    Parser::from_str(input).parse_parameters(&mut params)?;
    assert_eq!(expected, params);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_params_with_spaces() -> Result<(), Error> {
    let input = "; key1=?0; key2=11111";
    let expected = Parameters::from_iter(vec![
        (key_ref("key1").to_owned(), BareItem::Boolean(false)),
        (key_ref("key2").to_owned(), 11111.into()),
    ]);
    let mut params = Parameters::new();
    Parser::from_str(input).parse_parameters(&mut params)?;
    assert_eq!(expected, params);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_params_empty() -> Result<(), Error> {
    let mut params = Parameters::new();
    Parser::from_str(" key1=?0; key2=11111").parse_parameters(&mut params)?;
    assert_eq!(Parameters::new(), params);
    Parser::from_str("").parse_parameters(&mut params)?;
    assert_eq!(Parameters::new(), params);
    Parser::from_str("[;a=1").parse_parameters(&mut params)?;
    assert_eq!(Parameters::new(), params);
    Parser::from_str("").parse_parameters(&mut params)?;
    assert_eq!(Parameters::new(), params);
    Ok(())
}

#[test]
fn parse_key() -> Result<(), Error> {
    assert_eq!(key_ref("a"), Parser::from_str("a=1").parse_key()?);
    assert_eq!(key_ref("a1"), Parser::from_str("a1=10").parse_key()?);
    assert_eq!(key_ref("*1"), Parser::from_str("*1=10").parse_key()?);
    assert_eq!(key_ref("f"), Parser::from_str("f[f=10").parse_key()?);
    Ok(())
}

#[test]
fn parse_key_errors() {
    assert_eq!(
        Err(Error::with_index(
            "expected start of key ('a'-'z' or '*')",
            0
        )),
        Parser::from_str("[*f=10").parse_key()
    );
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_more_list() -> Result<(), Error> {
    let item1 = Item::new(1);
    let item2 = Item::new(2);
    let item3 = Item::new(42);
    let inner_list_1 = InnerList::new(vec![item1, item2]);
    let expected_list: List = vec![inner_list_1.into(), item3.into()];

    let mut parsed_header = Parser::from_str("(1 2)").parse_list()?;
    Parser::from_str("42").parse_list_with_visitor(&mut parsed_header)?;
    assert_eq!(expected_list, parsed_header);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_more_dict() -> Result<(), Error> {
    let item2_params = Parameters::from_iter(vec![(
        key_ref("foo").to_owned(),
        BareItem::Token(token_ref("*").to_owned()),
    )]);
    let item1 = Item::new(1);
    let item2 = Item::with_params(true, item2_params);
    let item3 = Item::new(3);
    let expected_dict = Dictionary::from_iter(vec![
        (key_ref("a").to_owned(), item1.into()),
        (key_ref("b").to_owned(), item2.into()),
        (key_ref("c").to_owned(), item3.into()),
    ]);

    let mut parsed_header = Parser::from_str("a=1, b;foo=*\t\t").parse_dictionary()?;
    Parser::from_str(" c=3").parse_dictionary_with_visitor(&mut parsed_header)?;
    assert_eq!(expected_dict, parsed_header);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_more_errors() -> Result<(), Error> {
    let mut parsed_dict_header = Parser::from_str("a=1, b;foo=*").parse_dictionary()?;
    assert!(Parser::from_str(",a")
        .parse_dictionary_with_visitor(&mut parsed_dict_header)
        .is_err());

    let mut parsed_list_header = Parser::from_str("a, b;foo=*").parse_list()?;
    assert!(Parser::from_str("(a, 2)")
        .parse_list_with_visitor(&mut parsed_list_header)
        .is_err());
    Ok(())
}
