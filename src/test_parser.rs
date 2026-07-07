use std::convert::Infallible;

use crate::{
    error, key_ref, token_ref,
    visitor::{
        DictionaryVisitor, EntryVisitor, Ignored, InnerListVisitor, ItemVisitor, ListVisitor,
        ParameterVisitor,
    },
    BareItemFromInput, Error, KeyRef, Parser,
};
#[cfg(feature = "parsed-types")]
use crate::{BareItem, Date, Dictionary, InnerList, Item, List, ListEntry, Parameters, Version};

#[cfg(feature = "parsed-types")]
macro_rules! item {
    ($val:expr) => {
        Item::from($val)
    };
    ($val:expr; {$($k:expr => $v:expr),* $(,)?}) => {
        Item::with_params($val, params!({$($k => $v),*}))
    };
}

#[cfg(feature = "parsed-types")]
macro_rules! params {
    ({$($k:expr => $v:expr),* $(,)?}) => {
        Parameters::from([
            $( (key_ref($k).to_owned(), BareItem::from($v)) ),*
        ])
    };
}

#[cfg(feature = "parsed-types")]
macro_rules! inner_list {
    ($($item:expr),* $(,)?) => {
        InnerList::new(vec![$( Item::from($item) ),*])
    };
    ([$($item:expr),* $(,)?]; {$($k:expr => $v:expr),* $(,)?}) => {
        InnerList::with_params(
            vec![$( Item::from($item) ),*],
            params!({$($k => $v),*})
        )
    };
}

#[cfg(feature = "parsed-types")]
macro_rules! list {
    ($($v:expr),* $(,)?) => {
        vec![$( ListEntry::from($v) ),*]
    };
}

#[cfg(feature = "parsed-types")]
macro_rules! dict {
    ($($k:expr => $v:expr),* $(,)?) => {
        Dictionary::from([
            $( (key_ref($k).to_owned(), ListEntry::from($v)) ),*
        ])
    };
}

#[test]
fn parse_errors() {
    let input = r#""some_value¢""#;
    assert_eq!(
        Err(error::Repr::InvalidStringCharacter(11).into()),
        Parser::new(input).parse_item_with_visitor(Ignored)
    );
    let input = r#""some_value" trailing_text""#;
    assert_eq!(
        Err(error::Repr::TrailingCharactersAfterParsedValue(13).into()),
        Parser::new(input).parse_item_with_visitor(Ignored)
    );
    assert_eq!(
        Err(error::Repr::ExpectedStartOfBareItem(0).into()),
        Parser::new("").parse_item_with_visitor(Ignored)
    );
}

#[test]
fn parse_list_errors() {
    let input = ",";
    assert_eq!(
        Err(error::Repr::ExpectedStartOfBareItem(0).into()),
        Parser::new(input).parse_list_with_visitor(Ignored)
    );

    let input = "a, b c";
    assert_eq!(
        Err(error::Repr::TrailingCharactersAfterMember(5).into()),
        Parser::new(input).parse_list_with_visitor(Ignored)
    );

    let input = "a,";
    assert_eq!(
        Err(error::Repr::TrailingComma(1).into()),
        Parser::new(input).parse_list_with_visitor(Ignored)
    );

    let input = "a     ,    ";
    assert_eq!(
        Err(error::Repr::TrailingComma(6).into()),
        Parser::new(input).parse_list_with_visitor(Ignored)
    );

    let input = "a\t \t ,\t ";
    assert_eq!(
        Err(error::Repr::TrailingComma(5).into()),
        Parser::new(input).parse_list_with_visitor(Ignored)
    );

    let input = "a\t\t,\t\t\t";
    assert_eq!(
        Err(error::Repr::TrailingComma(3).into()),
        Parser::new(input).parse_list_with_visitor(Ignored)
    );

    let input = "(a b),";
    assert_eq!(
        Err(error::Repr::TrailingComma(5).into()),
        Parser::new(input).parse_list_with_visitor(Ignored)
    );

    let input = "(1, 2, (a b)";
    assert_eq!(
        Err(error::Repr::ExpectedInnerListDelimiter(2).into()),
        Parser::new(input).parse_list_with_visitor(Ignored)
    );
}

#[test]
fn parse_inner_list_errors() {
    let input = "c b); a=1";
    assert_eq!(
        Err(error::Repr::ExpectedStartOfInnerList(0)),
        Parser::new(input).parse_inner_list(Ignored)
    );

    let input = "(";
    assert_eq!(
        Err(error::Repr::UnterminatedInnerList(1)),
        Parser::new(input).parse_inner_list(Ignored)
    );
}

#[test]
fn parse_dict_errors() {
    let input = "abc=123;a=1;b=2 def";
    assert_eq!(
        Err(error::Repr::TrailingCharactersAfterMember(16).into()),
        Parser::new(input).parse_dictionary_with_visitor(Ignored)
    );
    let input = "abc=123;a=1,";
    assert_eq!(
        Err(error::Repr::TrailingComma(11).into()),
        Parser::new(input).parse_dictionary_with_visitor(Ignored)
    );
}

#[test]
fn parse_bare_item_errors() {
    assert_eq!(
        Err(error::Repr::ExpectedStartOfBareItem(0)),
        Parser::new("!?0").parse_bare_item()
    );
    assert_eq!(
        Err(error::Repr::ExpectedStartOfBareItem(0)),
        Parser::new("_11abc").parse_bare_item()
    );
    assert_eq!(
        Err(error::Repr::ExpectedStartOfBareItem(0)),
        Parser::new("   ").parse_bare_item()
    );
}

#[test]
fn parse_bool_errors() {
    assert_eq!(
        Err(error::Repr::ExpectedStartOfBoolean(0)),
        Parser::new("").parse_bool()
    );
    assert_eq!(
        Err(error::Repr::ExpectedBoolean(1)),
        Parser::new("?").parse_bool()
    );
}

#[test]
fn parse_string_errors() {
    assert_eq!(
        Err(error::Repr::ExpectedStartOfString(0)),
        Parser::new("test").parse_string()
    );
    assert_eq!(
        Err(error::Repr::UnterminatedEscapeSequence(2)),
        Parser::new(r#""\"#).parse_string()
    );
    assert_eq!(
        Err(error::Repr::InvalidEscapeSequence(2)),
        Parser::new(r#""\l""#).parse_string()
    );
    assert_eq!(
        Err(error::Repr::InvalidStringCharacter(1)),
        Parser::new("\"\u{1f}\"").parse_string()
    );
    assert_eq!(
        Err(error::Repr::UnterminatedString(5)),
        Parser::new(r#""smth"#).parse_string()
    );
}

#[test]
fn parse_token_errors() {
    let mut parser = Parser::new("765token");
    assert_eq!(
        Err(error::Repr::ExpectedStartOfToken(0)),
        parser.parse_token()
    );
    assert_eq!(parser.remaining(), b"765token");

    assert_eq!(
        Err(error::Repr::ExpectedStartOfToken(0)),
        Parser::new("7token").parse_token()
    );
    assert_eq!(
        Err(error::Repr::ExpectedStartOfToken(0)),
        Parser::new("").parse_token()
    );
}

#[test]
fn parse_byte_sequence_errors() {
    assert_eq!(
        Err(error::Repr::ExpectedStartOfByteSequence(0)),
        Parser::new("aGVsbG8").parse_byte_sequence()
    );
    assert_eq!(
        Err(error::Repr::InvalidByteSequence(6)),
        Parser::new(":aGVsb G8=:").parse_byte_sequence()
    );
    assert_eq!(
        Err(error::Repr::UnterminatedByteSequence(9)),
        Parser::new(":aGVsbG8=").parse_byte_sequence()
    );
}

#[test]
fn parse_number_errors() {
    let mut parser = Parser::new(":aGVsbG8:rest");
    assert_eq!(Err(error::Repr::ExpectedDigit(0)), parser.parse_number());
    assert_eq!(parser.remaining(), b":aGVsbG8:rest");

    let mut parser = Parser::new("-11.5555 test string");
    assert_eq!(
        Err(error::Repr::TooManyDigitsAfterDecimalPoint(7)),
        parser.parse_number()
    );
    assert_eq!(parser.remaining(), b"5 test string");

    assert_eq!(
        Err(error::Repr::ExpectedDigit(1)),
        Parser::new("--0").parse_number()
    );
    assert_eq!(
        Err(error::Repr::TooManyDigitsBeforeDecimalPoint(13)),
        Parser::new("1999999999999.1").parse_number()
    );
    assert_eq!(
        Err(error::Repr::TrailingDecimalPoint(11)),
        Parser::new("19888899999.").parse_number()
    );
    assert_eq!(
        Err(error::Repr::TooManyDigits(15)),
        Parser::new("1999999999999999").parse_number()
    );
    assert_eq!(
        Err(error::Repr::TooManyDigitsAfterDecimalPoint(15)),
        Parser::new("19999999999.99991").parse_number()
    );
    assert_eq!(
        Err(error::Repr::ExpectedDigit(1)),
        Parser::new("- 42").parse_number()
    );
    assert_eq!(
        Err(error::Repr::TrailingDecimalPoint(1)),
        Parser::new("1..4").parse_number()
    );
    assert_eq!(
        Err(error::Repr::ExpectedDigit(1)),
        Parser::new("-").parse_number()
    );
    assert_eq!(
        Err(error::Repr::TrailingDecimalPoint(2)),
        Parser::new("-5. 14").parse_number()
    );
    assert_eq!(
        Err(error::Repr::TrailingDecimalPoint(1)),
        Parser::new("7. 1").parse_number()
    );
    assert_eq!(
        Err(error::Repr::TooManyDigitsAfterDecimalPoint(6)),
        Parser::new("-7.3333333333").parse_number()
    );
    assert_eq!(
        Err(error::Repr::TooManyDigitsBeforeDecimalPoint(14)),
        Parser::new("-7333333333323.12").parse_number()
    );
}

#[test]
fn parse_key_errors() {
    assert_eq!(
        Err(error::Repr::ExpectedStartOfKey(0)),
        Parser::new("[*f=10").parse_key()
    );
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_more_list() -> Result<(), Error> {
    let expected_list = list![inner_list![1, 2], 42];

    let mut parsed_header: List = Parser::new("(1 2)").parse()?;
    Parser::new("42").parse_list_with_visitor(&mut parsed_header)?;
    assert_eq!(expected_list, parsed_header);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_more_dict() -> Result<(), Error> {
    let expected_dict = dict! {
        "a" => 1,
        "b" => item!(true; {"foo" => token_ref("*")}),
        "c" => 3
    };

    let mut parsed_header: Dictionary = Parser::new("a=1, b;foo=*\t\t").parse()?;
    Parser::new(" c=3").parse_dictionary_with_visitor(&mut parsed_header)?;
    assert_eq!(expected_dict, parsed_header);
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_more_errors() -> Result<(), Error> {
    let mut parsed_dict_header: Dictionary = Parser::new("a=1, b;foo=*").parse()?;
    assert!(Parser::new(",a")
        .parse_dictionary_with_visitor(&mut parsed_dict_header)
        .is_err());

    let mut parsed_list_header: List = Parser::new("a, b;foo=*").parse()?;
    assert!(Parser::new("(a, 2)")
        .parse_list_with_visitor(&mut parsed_list_header)
        .is_err());
    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_date() -> Result<(), Error> {
    let input = "@0";

    assert!(Parser::new(input)
        .with_version(Version::Rfc8941)
        .parse::<Item>()
        .is_err());

    assert_eq!(Parser::new(input).parse::<Item>()?, item!(Date::UNIX_EPOCH));

    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_display_string() -> Result<(), Error> {
    let input = r#"%"This is intended for display to %c3%bcsers.""#;

    assert!(Parser::new(input)
        .with_version(Version::Rfc8941)
        .parse::<Item>()
        .is_err());

    assert_eq!(
        Parser::new(input).parse::<Item>()?,
        item!(BareItem::DisplayString(
            "This is intended for display to üsers.".to_owned()
        ))
    );

    Ok(())
}

#[test]
fn parse_display_string_errors() {
    assert_eq!(
        Parser::new(" %").parse_item_with_visitor(Ignored),
        Err(error::Repr::ExpectedQuote(2).into())
    );

    assert_eq!(
        Parser::new(r#" %""#).parse_item_with_visitor(Ignored),
        Err(error::Repr::UnterminatedDisplayString(3).into())
    );

    assert_eq!(
        Parser::new(r#" %"%"#).parse_item_with_visitor(Ignored),
        Err(error::Repr::UnterminatedEscapeSequence(4).into())
    );

    assert_eq!(
        Parser::new(r#" %"%a"#).parse_item_with_visitor(Ignored),
        Err(error::Repr::UnterminatedEscapeSequence(5).into())
    );

    assert_eq!(
        Parser::new(r#" %"%A"#).parse_item_with_visitor(Ignored),
        Err(error::Repr::InvalidEscapeSequence(4).into())
    );

    assert_eq!(
        Parser::new(r#" %"%aA"#).parse_item_with_visitor(Ignored),
        Err(error::Repr::InvalidEscapeSequence(5).into())
    );

    assert_eq!(
        Parser::new(r#" %"x%aa""#).parse_item_with_visitor(Ignored),
        Err(error::Repr::InvalidUtf8InDisplayString(4).into())
    );
}

/// A simple struct used for the complex tests.
#[derive(Default, Debug, PartialEq)]
struct Point {
    x: i64,
    y: i64,
}

impl Point {
    fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }
}

// For when a `Point` is a parameter somewhere.
impl<'de> ParameterVisitor<'de> for &mut Point {
    type Out = ();
    type Error = Infallible;

    fn parameter(
        &mut self,
        key: &'de KeyRef,
        value: BareItemFromInput<'de>,
    ) -> Result<(), Self::Error> {
        let Some(v) = value.as_integer() else {
            return Ok(());
        };
        let ptr = match key.as_str() {
            "x" => &mut self.x,
            "y" => &mut self.y,
            _ => return Ok(()),
        };
        *ptr = i64::from(v);
        Ok(())
    }

    fn finish(self) -> Result<Self::Out, Self::Error> {
        Ok(())
    }
}

impl<'de> DictionaryVisitor<'de> for &mut Point {
    type Out = ();
    type Error = Infallible;

    fn entry(&mut self, key: &'de KeyRef) -> Result<impl EntryVisitor<'de>, Self::Error> {
        let coord = match key.as_str() {
            "x" => &mut self.x,
            "y" => &mut self.y,
            _ => return Ok(None),
        };
        Ok(Some(CoordVisitor { coord }))
    }

    fn finish(self) -> Result<Self::Out, Self::Error> {
        Ok(())
    }
}

struct CoordVisitor<'a> {
    coord: &'a mut i64,
}

impl<'de> ItemVisitor<'de> for CoordVisitor<'_> {
    type Out = ();
    type Error = Infallible;

    fn bare_item(
        self,
        bare_item: BareItemFromInput<'de>,
    ) -> Result<impl ParameterVisitor<'de, Out = Self::Out>, Self::Error> {
        if let Some(v) = bare_item.as_integer() {
            *self.coord = i64::from(v);
        }
        Ok(Ignored)
    }
}

impl<'de> EntryVisitor<'de> for CoordVisitor<'_> {
    type Error = Infallible;

    fn item(self) -> Result<impl ItemVisitor<'de>, Self::Error> {
        Ok(self)
    }

    fn inner_list(self) -> Result<impl InnerListVisitor<'de>, Self::Error> {
        Ok(Ignored)
    }
}

#[test]
fn complex_dict_visitor() {
    let mut point = Point::default();
    Parser::new("x=10, y=3")
        .parse_dictionary_with_visitor(&mut point)
        .expect("successful parse");
    assert_eq!(point, Point::new(10, 3));
}

// An item that is an integer, with optional `x` and `y` parameters.
#[derive(Default, Debug, PartialEq)]
struct Holder {
    v: i64,
    point: Point,
}

impl<'de> ItemVisitor<'de> for &mut Holder {
    type Out = Option<()>;
    type Error = Infallible;

    fn bare_item(
        self,
        bare_item: BareItemFromInput<'de>,
    ) -> Result<impl ParameterVisitor<'de, Out = Self::Out>, Self::Error> {
        Ok(if let Some(v) = bare_item.as_integer() {
            self.v = i64::from(v);
            Some(&mut self.point)
        } else {
            None
        })
    }
}

#[test]
fn complex_item_visitor() {
    let mut holder = Holder::default();
    Parser::new("12;x=7;y=-5")
        .parse_item_with_visitor(&mut holder)
        .expect("successful parse");
    assert_eq!(holder.point, Point::new(7, -5));
}

#[test]
fn complex_list_visitor() {
    #[derive(Default, Debug, PartialEq)]
    struct ListHolder {
        list: Vec<Holder>,
        point: Point,
    }

    impl<'de> ListVisitor<'de> for Vec<ListHolder> {
        type Out = Self;
        type Error = Infallible;

        fn entry(&mut self) -> Result<impl EntryVisitor<'de>, Self::Error> {
            self.push(ListHolder::default());
            Ok(self.last_mut().unwrap()) // cannot fail
        }

        fn finish(self) -> Result<Self::Out, Self::Error> {
            Ok(self)
        }
    }

    impl<'de> EntryVisitor<'de> for &mut ListHolder {
        type Error = Infallible;

        fn item(self) -> Result<impl ItemVisitor<'de>, Self::Error> {
            Ok(Ignored)
        }

        fn inner_list(self) -> Result<impl InnerListVisitor<'de>, Self::Error> {
            Ok(self)
        }
    }

    impl<'de> InnerListVisitor<'de> for &mut ListHolder {
        type Error = Infallible;

        fn item(&mut self) -> Result<impl ItemVisitor<'de>, Self::Error> {
            self.list.push(Holder::default());
            Ok(self.list.last_mut().unwrap()) // cannot fail
        }

        fn finish(self) -> Result<impl ParameterVisitor<'de>, Self::Error> {
            Ok(&mut self.point)
        }
    }

    let list: Vec<ListHolder> = Parser::new("(1;x=4 2;y=5 3);x=1;y=2,(4;x=12;y=33), ()")
        .parse_list()
        .expect("successful parse");

    let expected = vec![
        ListHolder {
            list: vec![
                Holder {
                    v: 1,
                    point: Point::new(4, 0),
                },
                Holder {
                    v: 2,
                    point: Point::new(0, 5),
                },
                Holder {
                    v: 3,
                    point: Point::default(),
                },
            ],
            point: Point::new(1, 2),
        },
        ListHolder {
            list: vec![Holder {
                v: 4,
                point: Point::new(12, 33),
            }],
            point: Point::default(),
        },
        ListHolder {
            list: Vec::new(),
            point: Point::default(),
        },
    ];

    assert_eq!(list, expected);
}

// Regression test for https://github.com/undef1nd/sfv/issues/194.
// This test does not compile without the associated fix.
#[test]
fn parse_dictionary_lifetime() -> Result<(), Error> {
    struct Visitor<'de>(Option<&'de KeyRef>);

    impl<'de> DictionaryVisitor<'de> for Visitor<'de> {
        type Out = Option<&'de KeyRef>;
        type Error = Infallible;

        fn entry(&mut self, key: &'de KeyRef) -> Result<impl EntryVisitor<'de>, Self::Error> {
            self.0 = Some(key);
            Ok(Ignored)
        }

        fn finish(self) -> Result<Self::Out, Self::Error> {
            Ok(self.0)
        }
    }

    assert_eq!(
        Parser::new("a=1").parse_dictionary_with_visitor(Visitor(None))?,
        Some(key_ref("a"))
    );
    Ok(())
}
