#[cfg(feature = "parsed-types")]
use crate::{BareItem, Date, Item, Version};
use crate::{
    BareItemFromInput, Error, KeyRef, Parser, TokenRef, error, key_ref, token_ref,
    visitor::{
        BareItemVisitor, DictionaryVisitor, EntryVisitor, Ignored, InnerListVisitor, ItemVisitor,
        ListVisitor, ParameterVisitor,
    },
};

fn check_expected_errs<'de, const N: usize, T, E>(
    parse: impl Fn(Parser<'de>, Ignored) -> Result<T, E>,
    tests: [(&'de str, error::Repr); N],
) where
    T: PartialEq + std::fmt::Debug,
    E: Into<Error> + From<error::Repr> + std::fmt::Debug,
{
    for (input, expected_err) in tests {
        assert_eq!(
            Err(Error::from(expected_err)),
            parse(Parser::new(input), Ignored).map_err(Into::into),
            "{input}",
        );
    }
}

fn check_expected_errs_mut<'de, const N: usize, T>(
    parse: impl Fn(&mut Parser<'de>) -> Result<T, error::Repr>,
    tests: [(&'de str, error::Repr); N],
) where
    T: PartialEq + std::fmt::Debug,
{
    check_expected_errs(|ref mut parser, _| parse(parser), tests);
}

#[test]
fn parse_item_errors() {
    check_expected_errs(
        Parser::parse_item_with_visitor,
        [
            (r#""some_value¢""#, error::Repr::InvalidStringCharacter(11)),
            (
                r#""some_value" trailing_text""#,
                error::Repr::TrailingCharactersAfterParsedValue(13),
            ),
            ("", error::Repr::ExpectedStartOfBareItem(0)),
        ],
    );
}

#[test]
fn parse_list_errors() {
    check_expected_errs(
        Parser::parse_list_with_visitor,
        [
            (",", error::Repr::ExpectedStartOfBareItem(0)),
            ("a, b c", error::Repr::TrailingCharactersAfterMember(5)),
            ("a,", error::Repr::TrailingComma(1)),
            ("a     ,    ", error::Repr::TrailingComma(6)),
            ("a\t \t ,\t ", error::Repr::TrailingComma(5)),
            ("a\t\t,\t\t\t", error::Repr::TrailingComma(3)),
            ("(a b),", error::Repr::TrailingComma(5)),
            ("(1, 2, (a b)", error::Repr::ExpectedInnerListDelimiter(2)),
        ],
    );
}

#[test]
fn parse_inner_list_errors() {
    check_expected_errs_mut(
        |parser| parser.parse_inner_list(Ignored),
        [
            ("c b); a=1", error::Repr::ExpectedStartOfInnerList(0)),
            ("(", error::Repr::UnterminatedInnerList(1)),
        ],
    );
}

#[test]
fn parse_dict_errors() {
    check_expected_errs(
        Parser::parse_dictionary_with_visitor,
        [
            (
                "abc=123;a=1;b=2 def",
                error::Repr::TrailingCharactersAfterMember(16),
            ),
            ("abc=123;a=1,", error::Repr::TrailingComma(11)),
        ],
    );
}

#[test]
fn parse_bare_item_errors() {
    check_expected_errs_mut(
        Parser::parse_bare_item,
        [
            ("!?0", error::Repr::ExpectedStartOfBareItem(0)),
            ("_11abc", error::Repr::ExpectedStartOfBareItem(0)),
            ("   ", error::Repr::ExpectedStartOfBareItem(0)),
        ],
    );
}

#[test]
fn parse_bool_errors() {
    check_expected_errs_mut(
        Parser::parse_bool,
        [
            ("", error::Repr::ExpectedStartOfBoolean(0)),
            ("?", error::Repr::ExpectedBoolean(1)),
        ],
    );
}

#[test]
fn parse_string_errors() {
    check_expected_errs_mut(
        Parser::parse_string,
        [
            ("test", error::Repr::ExpectedStartOfString(0)),
            (r#""\"#, error::Repr::UnterminatedEscapeSequence(2)),
            (r#""\l""#, error::Repr::InvalidEscapeSequence(2)),
            ("\"\u{1f}\"", error::Repr::InvalidStringCharacter(1)),
            (r#""smth"#, error::Repr::UnterminatedString(5)),
        ],
    );
}

#[test]
fn parse_token_errors() {
    check_expected_errs_mut(
        Parser::parse_token,
        [
            ("765token", error::Repr::ExpectedStartOfToken(0)),
            ("7token", error::Repr::ExpectedStartOfToken(0)),
            ("", error::Repr::ExpectedStartOfToken(0)),
        ],
    );
}

#[test]
fn parse_byte_sequence_errors() {
    check_expected_errs_mut(
        Parser::parse_byte_sequence,
        [
            ("aGVsbG8", error::Repr::ExpectedStartOfByteSequence(0)),
            (":aGVsb G8=:", error::Repr::InvalidByteSequence(6)),
            (":aGVsbG8=", error::Repr::UnterminatedByteSequence(9)),
        ],
    );
}

#[test]
fn parse_number_errors() {
    check_expected_errs_mut(
        Parser::parse_number,
        [
            (":aGVsbG8:rest", error::Repr::ExpectedDigit(0)),
            (
                "-11.5555 test string",
                error::Repr::TooManyDigitsAfterDecimalPoint(7),
            ),
            ("--0", error::Repr::ExpectedDigit(1)),
            (
                "1999999999999.1",
                error::Repr::TooManyDigitsBeforeDecimalPoint(13),
            ),
            ("19888899999.", error::Repr::TrailingDecimalPoint(11)),
            ("1999999999999999", error::Repr::TooManyDigits(15)),
            (
                "19999999999.99991",
                error::Repr::TooManyDigitsAfterDecimalPoint(15),
            ),
            ("- 42", error::Repr::ExpectedDigit(1)),
            ("1..4", error::Repr::TrailingDecimalPoint(1)),
            ("-", error::Repr::ExpectedDigit(1)),
            ("-5. 14", error::Repr::TrailingDecimalPoint(2)),
            ("7. 1", error::Repr::TrailingDecimalPoint(1)),
            (
                "-7.3333333333",
                error::Repr::TooManyDigitsAfterDecimalPoint(6),
            ),
            (
                "-7333333333323.12",
                error::Repr::TooManyDigitsBeforeDecimalPoint(14),
            ),
        ],
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
fn parse_date() -> Result<(), Error> {
    let input = "@0";

    assert!(
        Parser::new(input)
            .with_version(Version::Rfc8941)
            .parse::<Item>()
            .is_err()
    );

    assert_eq!(
        Parser::new(input).parse::<Item>()?,
        Item::from(Date::UNIX_EPOCH)
    );

    Ok(())
}

#[test]
#[cfg(feature = "parsed-types")]
fn parse_display_string() -> Result<(), Error> {
    let input = r#"%"This is intended for display to %c3%bcsers.""#;

    assert!(
        Parser::new(input)
            .with_version(Version::Rfc8941)
            .parse::<Item>()
            .is_err()
    );

    for (raw, expected) in [
        (input, "This is intended for display to üsers."),
        (r#"%"""#, ""),
        (r#"%"hello""#, "hello"),
        (r#"%"%c3%bc""#, "ü"),
        (r#"%"%c3%bc mid %25 end %e2%82%ac""#, "ü mid % end €"),
    ] {
        assert_eq!(
            Parser::new(raw).parse::<Item>()?,
            Item::from(BareItem::DisplayString(expected.to_owned())),
        );
    }

    Ok(())
}

#[test]
fn parse_display_string_errors() {
    check_expected_errs(
        Parser::parse_item_with_visitor,
        [
            (" %", error::Repr::ExpectedQuote(2)),
            (r#" %""#, error::Repr::UnterminatedDisplayString(3)),
            (r#" %"%"#, error::Repr::UnterminatedEscapeSequence(4)),
            (r#" %"%a"#, error::Repr::UnterminatedEscapeSequence(5)),
            (r#" %"%A"#, error::Repr::InvalidEscapeSequence(4)),
            (r#" %"%aA"#, error::Repr::InvalidEscapeSequence(5)),
            (r#" %"x%aa""#, error::Repr::InvalidUtf8InDisplayString(4)),
            (
                r#" %"%c3%bc%aa""#,
                error::Repr::InvalidUtf8InDisplayString(9),
            ),
        ],
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

    fn coord(&mut self, key: &KeyRef) -> Option<CoordVisitor<'_>> {
        let coord = match key.as_str() {
            "x" => &mut self.x,
            "y" => &mut self.y,
            _ => return None,
        };
        Some(CoordVisitor { coord })
    }
}

// For when a `Point` is a parameter somewhere.
impl<'de> ParameterVisitor<'de> for &mut Point {
    type Out = ();

    fn parameter(&mut self, key: &'de KeyRef) -> impl BareItemVisitor<'de> {
        self.coord(key)
    }

    fn finish(self) -> Self::Out {}
}

impl<'de> DictionaryVisitor<'de> for &mut Point {
    type Out = ();

    fn entry(&mut self, key: &'de KeyRef) -> impl EntryVisitor<'de> {
        self.coord(key)
    }

    fn finish(self) -> Self::Out {}
}

struct CoordVisitor<'a> {
    coord: &'a mut i64,
}

impl<'de> BareItemVisitor<'de> for CoordVisitor<'_> {
    type Out = Ignored;

    fn bare_item(self, bare_item: BareItemFromInput<'de>) -> Self::Out {
        *self.coord = bare_item.as_integer().map_or(0, i64::from);
        Ignored
    }
}

impl<'de> EntryVisitor<'de> for CoordVisitor<'_> {
    fn item(self) -> impl ItemVisitor<'de> {
        self
    }

    fn inner_list(self) -> impl InnerListVisitor<'de> {
        *self.coord = 0;
        Ignored
    }
}

#[test]
fn complex_dict_visitor() {
    let mut point = Point::default();
    Parser::new("x=10, y=3")
        .parse_dictionary_with_visitor(&mut point)
        .expect("successful parse");
    assert_eq!(point, Point::new(10, 3));

    let mut point = Point::default();
    Parser::new("x=10, y=3, x=foo, y=()")
        .parse_dictionary_with_visitor(&mut point)
        .expect("successful parse");
    assert_eq!(point, Point::new(0, 0));
}

// An item that is an integer, with optional `x` and `y` parameters.
#[derive(Default, Debug, PartialEq)]
struct Holder {
    v: i64,
    point: Point,
}

impl<'a, 'de> BareItemVisitor<'de> for &'a mut Holder {
    type Out = Option<&'a mut Point>;

    fn bare_item(self, bare_item: BareItemFromInput<'de>) -> Self::Out {
        if let Some(v) = bare_item.as_integer() {
            self.v = i64::from(v);
            Some(&mut self.point)
        } else {
            None
        }
    }
}

#[test]
fn complex_item_visitor() {
    let mut holder = Holder::default();
    Parser::new("12;x=7;y=-5")
        .parse_item_with_visitor(&mut holder)
        .expect("successful parse");
    assert_eq!(holder.point, Point::new(7, -5));

    let mut holder = Holder::default();
    Parser::new("12;x=7;y=-5;x=foo")
        .parse_item_with_visitor(&mut holder)
        .expect("successful parse");
    assert_eq!(holder.point, Point::new(0, -5));
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

        fn entry(&mut self) -> impl EntryVisitor<'de> {
            self.push_mut(ListHolder::default())
        }

        fn finish(self) -> Self::Out {
            self
        }
    }

    impl<'de> EntryVisitor<'de> for &mut ListHolder {
        fn item(self) -> impl ItemVisitor<'de> {
            Ignored
        }

        fn inner_list(self) -> impl InnerListVisitor<'de> {
            self
        }
    }

    impl<'de> InnerListVisitor<'de> for &mut ListHolder {
        fn item(&mut self) -> impl ItemVisitor<'de> {
            self.list.push_mut(Holder::default())
        }

        fn finish(self) -> impl ParameterVisitor<'de> {
            &mut self.point
        }
    }

    let list = Parser::new("(1;x=4 2;y=5 3);x=1;y=2,(4;x=12;y=33), ()")
        .parse_list_with_visitor(Vec::new())
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

        fn entry(&mut self, key: &'de KeyRef) -> impl EntryVisitor<'de> {
            self.0 = Some(key);
            Ignored
        }

        fn finish(self) -> Self::Out {
            self.0
        }
    }

    assert_eq!(
        Parser::new("a=1").parse_dictionary_with_visitor(Visitor(None))?,
        Some(key_ref("a"))
    );
    Ok(())
}

#[test]
fn fallible_domain_validation() {
    // Domain error type intentionally does not implement `std::error::Error`.
    #[derive(Debug, PartialEq)]
    enum ListError {
        ExpectedToken,
        TooManyTokens,
    }

    struct TokenListVisitor<'de>(Result<Vec<&'de TokenRef>, ListError>);

    struct TokenEntry<'a, 'de>(&'a mut Result<Vec<&'de TokenRef>, ListError>);

    impl<'de> EntryVisitor<'de> for TokenEntry<'_, 'de> {
        fn item(self) -> impl ItemVisitor<'de> {
            move |bare_item: BareItemFromInput<'de>| {
                if let BareItemFromInput::Token(tok) = bare_item {
                    if let Ok(vec) = self.0 {
                        vec.push(tok);
                    }
                } else {
                    *self.0 = Err(ListError::ExpectedToken);
                }
                Ignored
            }
        }

        fn inner_list(self) -> impl InnerListVisitor<'de> {
            *self.0 = Err(ListError::ExpectedToken);
            Ignored
        }
    }

    impl<'de> ListVisitor<'de> for TokenListVisitor<'de> {
        type Out = Result<Vec<&'de TokenRef>, ListError>;

        fn entry(&mut self) -> impl EntryVisitor<'de> {
            let Ok(vec) = &self.0 else {
                return None;
            };
            if vec.len() == 2 {
                self.0 = Err(ListError::TooManyTokens);
                return None;
            }
            Some(TokenEntry(&mut self.0))
        }

        fn finish(self) -> Self::Out {
            self.0
        }
    }

    let make_visitor = || TokenListVisitor(Ok(Vec::new()));

    assert_eq!(
        Parser::new("a, b").parse_list_with_visitor(make_visitor()),
        Ok(Ok(vec![token_ref("a"), token_ref("b")])),
    );
    assert_eq!(
        Parser::new("123, b").parse_list_with_visitor(make_visitor()),
        Ok(Err(ListError::ExpectedToken)),
    );
    assert_eq!(
        Parser::new("(), b").parse_list_with_visitor(make_visitor()),
        Ok(Err(ListError::ExpectedToken)),
    );
    assert_eq!(
        Parser::new("a, b, c").parse_list_with_visitor(make_visitor()),
        Ok(Err(ListError::TooManyTokens)),
    );
    assert_eq!(
        Parser::new("123, ?2").parse_list_with_visitor(make_visitor()),
        Err(error::Repr::ExpectedBoolean(6).into()),
    );
}
