/*!
Contains traits for parsing structured-field values incrementally.

# Design: Infallible Visitation

Visitor trait methods are intentionally infallible and cannot short-circuit
parsing early:

- **Strict syntactic validation ([RFC 9651 Section 1.2], [Section 4.2]):**
  Structured-field parsers must reject the entire field value if *any* part of
  the input is syntactically malformed—even parts whose values are semantically
  ignored or follow an earlier semantic error. Aborting mid-parse on a visitor
  error would leave trailing input unparsed and misclassify syntax errors (such
  as `123;0` when expecting a token) as semantic errors.
- **Duplicate dictionary and parameter keys ([RFC 9651 Section 4.2.2], [Section 4.2.6]):**
  When duplicate keys appear in the same dictionary or parameter list, all but
  the last instance must be ignored. Preventing mid-parse failure ensures an
  invalid value on an earlier occurrence of a key (such as `color=123, color=red`
  or `color=red;strong=a;strong`) cannot prematurely abort parsing before a
  later valid occurrence overrides it.
- **Incremental execution before completion:** Visitor methods run *during*
  parsing before the full input is known to be syntactically valid.
  Implementations should avoid external side effects or expensive work during
  visitation (performing them only after parsing succeeds), and must ensure
  later occurrences of duplicate dictionary or parameter keys overwrite earlier
  state (including any recorded semantic error).

To perform fallible domain validation, accumulate validation state in the
visitor (such as `Result<T, MyError>`) and return it as the top-level `Out` type
([`DictionaryVisitor::Out`], [`ListVisitor::Out`], or
[`ParameterVisitor::Out`]). [`Parser::parse_dictionary_with_visitor`][crate::Parser::parse_dictionary_with_visitor]
(and its list/item counterparts) will return `Result<Result<T, MyError>, sfv::Error>`,
preserving typed domain errors without requiring [`std::error::Error`] or string
allocation. Use [`Ignored`] or [`Option<V>`] (`None`) to skip visiting irrelevant
or already-invalidated subtrees while still syntactically validating the
underlying input.

[RFC 9651 Section 1.2]: <https://httpwg.org/specs/rfc9651.html#strict>
[Section 4.2]: <https://httpwg.org/specs/rfc9651.html#text-parse>
[RFC 9651 Section 4.2.2]: <https://httpwg.org/specs/rfc9651.html#parse-dictionary>
[Section 4.2.6]: <https://httpwg.org/specs/rfc9651.html#parse-param>

# Example

Consider a fictitious dictionary header `Foo` defined to contain a single
required top-level key, `color`, which is either the token `blue` or the token
`red`, with an optional boolean parameter `strong`, defaulting to `false`. If
`color` or `strong` has the wrong type, the entire header is considered invalid.

```
use sfv::{BareItemFromInput, KeyRef, visitor};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Color {
    Red,
    Blue,
}

struct InvalidColor;

impl TryFrom<&sfv::TokenRef> for Color {
    type Error = InvalidColor;

    fn try_from(v: &sfv::TokenRef) -> Result<Self, Self::Error> {
        match v.as_str() {
            "red" => Ok(Self::Red),
            "blue" => Ok(Self::Blue),
            _ => Err(InvalidColor),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Foo {
    color: Color,
    strong: bool,
}

#[derive(Debug)]
pub enum Error {
    ColorMissing,
    ColorWrongType,
    ColorInvalid,
    StrongWrongType,
    Syntax(sfv::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ColorMissing => f.write_str("color missing"),
            Self::ColorWrongType => f.write_str("color wrong type"),
            Self::ColorInvalid => f.write_str("color invalid"),
            Self::StrongWrongType => f.write_str("strong wrong type"),
            Self::Syntax(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for Error {}

const KEY_COLOR: &KeyRef = sfv::key_ref("color");
const KEY_STRONG: &KeyRef = sfv::key_ref("strong");

// Notes for real-world implementations:
//
// - Accept `&[u8]` rather than `&str`, since HTTP header field values are not
//   necessarily UTF-8.
// - Encapsulate parsing in a function rather than exposing
//   `TryFrom<&sfv::Dictionary>` or visitor types publicly, which could lead
//   callers to configure `sfv::Parser` with the wrong RFC version for the
//   header.
// - Expose only one of the two implementations below, which trade off
//   allocations, source-code complexity, and monomorphization costs:
//   - `parse_from_dictionary` is simpler to write and avoids monomorphizing a
//     custom visitor, but requires the `parsed-types` Cargo feature (which
//     entails an additional crate dependency) and allocates an intermediate
//     `Dictionary`.
//   - `parse_with_visitor` involves more source code and monomorphization, but
//     records only the minimal state needed to parse and validate the header
//     (avoiding the intermediate `Dictionary` allocation) and allows the
//     `parsed-types` Cargo feature to be disabled.

#[cfg(feature = "parsed-types")]
pub fn parse_from_dictionary(input: &[u8]) -> Result<Foo, Error> {
    let dict: sfv::Dictionary = sfv::Parser::new(input).parse().map_err(Error::Syntax)?;

    let Some(v) = dict.get(KEY_COLOR) else {
        return Err(Error::ColorMissing);
    };

    let sfv::ListEntry::Item(sfv::Item { bare_item: sfv::BareItem::Token(v), params }) = v else {
        return Err(Error::ColorWrongType);
    };

    let Ok(color) = Color::try_from(&**v) else {
        return Err(Error::ColorInvalid);
    };

    let strong = match params.get(KEY_STRONG) {
        None => false,
        Some(sfv::BareItem::Boolean(v)) => *v,
        Some(_) => return Err(Error::StrongWrongType),
    };

    Ok(Foo { color, strong })
}

pub fn parse_with_visitor(input: &[u8]) -> Result<Foo, Error> {
    struct Visitor(Result<Foo, Error>);

    struct ParamVisitor<'a> {
        state: &'a mut Result<Foo, Error>,
        color: Color,
    }

    impl<'de> visitor::DictionaryVisitor<'de> for Visitor {
        type Out = Result<Foo, Error>;

        fn entry(&mut self, key: &'de KeyRef) -> impl visitor::EntryVisitor<'de> {
            if key == KEY_COLOR {
                Some(self)
            } else {
                // Ignore other keys.
                None
            }
        }

        fn finish(self) -> Self::Out {
            self.0
        }
    }

    impl<'de> visitor::EntryVisitor<'de> for &mut Visitor {
        fn item(self) -> impl visitor::ItemVisitor<'de> {
            self
        }

        fn inner_list(self) -> impl visitor::InnerListVisitor<'de> {
            // Record the error so a later `color` entry can still override it.
            self.0 = Err(Error::ColorWrongType);
            visitor::Ignored
        }
    }

    impl<'de, 'a> visitor::BareItemVisitor<'de> for &'a mut Visitor {
        type Out = Option<ParamVisitor<'a>>;

        fn bare_item(self, v: BareItemFromInput<'de>) -> Self::Out {
            if let BareItemFromInput::Token(v) = v {
                if let Ok(color) = Color::try_from(v) {
                    self.0 = Ok(Foo { color, strong: false });
                    // Visit the parameters.
                    return Some(ParamVisitor { state: &mut self.0, color });
                }

                self.0 = Err(Error::ColorInvalid);
            } else {
                self.0 = Err(Error::ColorWrongType);
            }

            // No need to visit the parameters if the type or value is invalid.
            None
        }
    }

    impl<'de> visitor::ParameterVisitor<'de> for ParamVisitor<'_> {
        type Out = ();

        fn parameter(&mut self, key: &'de KeyRef) -> impl visitor::BareItemVisitor<'de> {
            // Ignore other parameters.
            if key == KEY_STRONG {
                let state = &mut *self.state;
                let color = self.color;
                // Record any error so a later `strong` parameter on the same
                // item can still override it.
                Some(move |v: BareItemFromInput<'de>| {
                    *state = if let BareItemFromInput::Boolean(strong) = v {
                        Ok(Foo { color, strong })
                    } else {
                        Err(Error::StrongWrongType)
                    };
                })
            } else {
                None
            }
        }

        fn finish(self) -> Self::Out {}
    }

    sfv::Parser::new(input)
        .parse_dictionary_with_visitor(Visitor(Err(Error::ColorMissing)))
        .map_err(Error::Syntax)?
}

for (input, expected) in [
    (
        "color=red",
        Ok(Foo { color: Color::Red, strong: false }),
    ),
    (
        "color=blue;strong",
        Ok(Foo { color: Color::Blue, strong: true }),
    ),
    (
        "",
        Err(Error::ColorMissing),
    ),
    (
        "color=123",
        Err(Error::ColorWrongType),
    ),
    (
        "color=(red)",
        Err(Error::ColorWrongType),
    ),
    (
        "color=green",
        Err(Error::ColorInvalid),
    ),
    (
        "color=red;strong=a",
        Err(Error::StrongWrongType),
    ),
    // Cases involving duplicate keys:
    (
        "color=123;strong, color=red",
        Ok(Foo { color: Color::Red, strong: false }),
    ),
    (
        "color=(blue);strong, color=red",
        Ok(Foo { color: Color::Red, strong: false }),
    ),
    (
        "color=green, color=blue",
        Ok(Foo { color: Color::Blue, strong: false }),
    ),
    (
        "color=blue;strong=a, color=red",
        Ok(Foo { color: Color::Red, strong: false }),
    ),
    (
        "color=red;strong, color=red",
        Ok(Foo { color: Color::Red, strong: false }),
    ),
    (
        "color=red;strong=a;strong",
        Ok(Foo { color: Color::Red, strong: true }),
    ),
    (
        "color=red;strong;strong=a",
        Err(Error::StrongWrongType),
    ),
] {
    println!("{input}");

    let expected = expected.map_err(|err| err.to_string());

    #[cfg(feature = "parsed-types")]
    assert_eq!(
        parse_from_dictionary(input.as_bytes()).map_err(|err| err.to_string()),
        expected,
    );

    assert_eq!(
        parse_with_visitor(input.as_bytes()).map_err(|err| err.to_string()),
        expected,
    );
}

// Syntax errors are also reported, even after semantic errors:
#[cfg(feature = "parsed-types")]
assert!(matches!(parse_from_dictionary(b"color=?2"), Err(Error::Syntax(_))));
assert!(matches!(parse_with_visitor(b"color=?2"), Err(Error::Syntax(_))));
assert!(matches!(parse_with_visitor(b"color=green, ?2"), Err(Error::Syntax(_))));
```

# Returning a value from `ItemVisitor`

When a top-level item is parsed with
[`Parser::parse_item_with_visitor`][crate::Parser::parse_item_with_visitor],
the [`ParameterVisitor::Out`] of its parameter visitor is returned directly on
success:

```
use sfv::visitor::{BareItemVisitor, Ignored, ParameterVisitor};
use sfv::{BareItemFromInput, KeyRef, Parser, TokenRef, Version, token_ref};

#[derive(Debug)]
enum BarError {
    Syntax(sfv::Error),
    ExpectedToken,
}

fn parse_bar(input: &[u8]) -> Result<&TokenRef, BarError> {
    struct Params<'de>(&'de TokenRef);

    impl<'de> ParameterVisitor<'de> for Params<'de> {
        type Out = &'de TokenRef;

        fn parameter(&mut self, _key: &'de KeyRef) -> impl BareItemVisitor<'de> {
            Ignored
        }

        fn finish(self) -> Self::Out {
            self.0
        }
    }

    Parser::new(input)
        .with_version(Version::Rfc8941)
        .parse_item_with_visitor(|bare_item| match bare_item {
            BareItemFromInput::Token(token) => Some(Params(token)),
            _ => None,
        })
        .map_err(BarError::Syntax)?
        .ok_or(BarError::ExpectedToken)
}

assert_eq!(parse_bar(b"abc;foo=1").unwrap(), token_ref("abc"));
assert!(matches!(parse_bar(b"123"), Err(BarError::ExpectedToken)));
assert!(matches!(parse_bar(b"@1659578233"), Err(BarError::Syntax(_))));
assert!(matches!(parse_bar(b"abc;0"), Err(BarError::Syntax(_))));
assert!(matches!(parse_bar(b"123;0"), Err(BarError::Syntax(_))));
```

# Discarding irrelevant parts

Two kinds of helpers are provided for silently discarding structured-field
parts:

- [`Ignored`]: This type implements all of the visitor traits as no-ops, and can
  be used when a visitor implementation would unconditionally do nothing. An
  example of this is when an item's bare item needs to be validated, but its
  parameters do not (e.g. because the relevant field definition prescribes
  none and permits unknown ones).

- Blanket implementations of [`BareItemVisitor`], [`ParameterVisitor`],
  [`ItemVisitor`], [`EntryVisitor`], and [`InnerListVisitor`] for [`Option<V>`]
  where `V` implements that trait: These implementations act like `Ignored` when
  `self` is [`None`], and forward to `V`'s implementation when `self` is
  [`Some`]. These can be used when the visitor dynamically handles or ignores
  field parts. Examples include ignoring unknown dictionary or parameter keys,
  skipping parameters on an already-invalid item, or skipping remaining entries
  of an already-invalidated list.

Note that the discarded parts are still validated during parsing: syntactic
errors in the input still cause parsing to fail even when these helpers are
used, [as required by RFC 9651](https://httpwg.org/specs/rfc9651.html#strict).

The `Foo` and `parse_bar` examples above demonstrate usage of both kinds of
helpers.
*/

use crate::{BareItemFromInput, KeyRef};

/// A visitor whose methods are called during bare-item parsing.
///
/// The lifetime `'de` is the lifetime of the input.
///
/// When [`Self::Out`][BareItemVisitor::Out] implements [`ParameterVisitor`],
/// this trait also serves as an [`ItemVisitor`], which can be used with
/// [`Parser::parse_item_with_visitor`][crate::Parser::parse_item_with_visitor].
///
/// See [the module documentation](crate::visitor#design-infallible-visitation)
/// for why visitor methods are infallible and how to represent validation
/// errors.
pub trait BareItemVisitor<'de> {
    /// The return type of the [`BareItemVisitor::bare_item`] method.
    ///
    /// For bare-item visitors returned by [`ParameterVisitor::parameter`], this
    /// is typically `()`. For [`ItemVisitor`]s, this must implement
    /// [`ParameterVisitor`] in order to handle the item's parameters.
    /// See [the module documentation](crate::visitor#discarding-irrelevant-parts)
    /// for guidance on discarding parameters.
    type Out;

    /// Called after a bare item has been parsed.
    fn bare_item(self, value: BareItemFromInput<'de>) -> Self::Out;
}

impl<'de, F, Out> BareItemVisitor<'de> for F
where
    F: FnOnce(BareItemFromInput<'de>) -> Out,
{
    type Out = Out;

    fn bare_item(self, value: BareItemFromInput<'de>) -> Self::Out {
        self(value)
    }
}

/// A visitor whose methods are called during parameter parsing.
///
/// The lifetime `'de` is the lifetime of the input.
///
/// See [the module documentation](crate::visitor#design-infallible-visitation)
/// for why visitor methods are infallible and how to represent validation
/// errors.
pub trait ParameterVisitor<'de> {
    /// The return type of the [`ParameterVisitor::finish`] method.
    ///
    /// Many implementations will set this to `()`. See
    /// [the module documentation](crate::visitor#returning-a-value-from-itemvisitor)
    /// for an example that does not.
    type Out;

    /// Called after a parameter key has been parsed.
    ///
    /// The returned visitor is used to handle the associated bare-item value.
    /// See [the module documentation](crate::visitor#discarding-irrelevant-parts)
    /// for guidance on discarding parameters.
    ///
    /// Note: Per [RFC 9651], when duplicate parameter keys are encountered in
    /// the same scope, all but the last instance are ignored. Implementations
    /// of this trait must respect that requirement in order to comply with the
    /// specification. For example, if parameters are stored in a map, earlier
    /// values for a given parameter key must be overwritten by later ones.
    ///
    /// [RFC 9651]: <https://httpwg.org/specs/rfc9651.html#parse-param>
    fn parameter(&mut self, key: &'de KeyRef) -> impl BareItemVisitor<'de>;

    /// Called after all parameters have been parsed.
    fn finish(self) -> Self::Out;
}

/// A visitor whose methods are called during item parsing.
///
/// This trait is an alias for a [`BareItemVisitor`] whose
/// [`Out`][BareItemVisitor::Out] implements [`ParameterVisitor`], and is
/// automatically implemented for all such types.
///
/// The lifetime `'de` is the lifetime of the input.
///
/// Use this trait with
/// [`Parser::parse_item_with_visitor`][crate::Parser::parse_item_with_visitor].
pub trait ItemVisitor<'de>: BareItemVisitor<'de, Out: ParameterVisitor<'de>> {}

impl<'de, V> ItemVisitor<'de> for V where V: BareItemVisitor<'de, Out: ParameterVisitor<'de>> {}

/// A visitor whose methods are called during inner-list parsing.
///
/// The lifetime `'de` is the lifetime of the input.
///
/// See [the module documentation](crate::visitor#design-infallible-visitation)
/// for why visitor methods are infallible and how to represent validation
/// errors.
pub trait InnerListVisitor<'de> {
    /// Called before an item has been parsed.
    ///
    /// The returned visitor is used to handle the item.
    fn item(&mut self) -> impl ItemVisitor<'de>;

    /// Called after all inner-list items have been parsed.
    ///
    /// The returned visitor is used to handle the inner list's parameters.
    /// See [the module documentation](crate::visitor#discarding-irrelevant-parts)
    /// for guidance on discarding parameters.
    fn finish(self) -> impl ParameterVisitor<'de>;
}

/// A visitor whose methods are called during entry parsing.
///
/// The lifetime `'de` is the lifetime of the input.
///
/// See [the module documentation](crate::visitor#design-infallible-visitation)
/// for why visitor methods are infallible and how to represent validation
/// errors.
pub trait EntryVisitor<'de> {
    /// Called before an item has been parsed.
    ///
    /// The returned visitor is used to handle the item.
    fn item(self) -> impl ItemVisitor<'de>;

    /// Called before an inner list has been parsed.
    ///
    /// The returned visitor is used to handle the inner list.
    fn inner_list(self) -> impl InnerListVisitor<'de>;
}

/// A visitor whose methods are called during dictionary parsing.
///
/// The lifetime `'de` is the lifetime of the input.
///
/// Use this trait with
/// [`Parser::parse_dictionary_with_visitor`][crate::Parser::parse_dictionary_with_visitor].
///
/// See [the module documentation](crate::visitor#design-infallible-visitation)
/// for why visitor methods are infallible and how to represent validation
/// errors.
pub trait DictionaryVisitor<'de> {
    /// The return type of the [`DictionaryVisitor::finish`] method.
    type Out;

    /// Called after a dictionary key has been parsed.
    ///
    /// The returned visitor is used to handle the associated value.
    /// See [the module documentation](crate::visitor#discarding-irrelevant-parts)
    /// for guidance on discarding entries.
    ///
    /// Note: Per [RFC 9651], when duplicate dictionary keys are encountered in
    /// the same scope, all but the last instance are ignored. Implementations
    /// of this trait must respect that requirement in order to comply with the
    /// specification. For example, if dictionary entries are stored in a map,
    /// earlier values for a given dictionary key must be overwritten by later
    /// ones.
    ///
    /// [RFC 9651]: <https://httpwg.org/specs/rfc9651.html#parse-dictionary>
    fn entry(&mut self, key: &'de KeyRef) -> impl EntryVisitor<'de>;

    /// Called after all dictionary keys have been parsed.
    fn finish(self) -> Self::Out;
}

/// A visitor whose methods are called during list parsing.
///
/// The lifetime `'de` is the lifetime of the input.
///
/// Use this trait with
/// [`Parser::parse_list_with_visitor`][crate::Parser::parse_list_with_visitor].
///
/// See [the module documentation](crate::visitor#design-infallible-visitation)
/// for why visitor methods are infallible and how to represent validation
/// errors.
pub trait ListVisitor<'de> {
    /// The return type of the [`ListVisitor::finish`] method.
    type Out;

    /// Called before a list entry has been parsed.
    ///
    /// The returned visitor is used to handle the entry.
    fn entry(&mut self) -> impl EntryVisitor<'de>;

    /// Called after all list entries have been parsed.
    fn finish(self) -> Self::Out;
}

/// A visitor that can be used to silently discard structured-field parts.
///
/// Note that the discarded parts are still validated during parsing: syntactic
/// errors in the input still cause parsing to fail even when this type is used,
/// [as required by RFC 9651](https://httpwg.org/specs/rfc9651.html#strict).
///
/// See [the module documentation](crate::visitor#discarding-irrelevant-parts)
/// for example usage.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ignored;

impl<'de> BareItemVisitor<'de> for Ignored {
    type Out = Self;

    fn bare_item(self, _value: BareItemFromInput<'de>) -> Self::Out {
        Self
    }
}

impl<'de> ParameterVisitor<'de> for Ignored {
    type Out = ();

    fn parameter(&mut self, _key: &'de KeyRef) -> impl BareItemVisitor<'de> {
        Self
    }

    fn finish(self) -> Self::Out {}
}

impl<'de> EntryVisitor<'de> for Ignored {
    fn item(self) -> impl ItemVisitor<'de> {
        Ignored
    }

    fn inner_list(self) -> impl InnerListVisitor<'de> {
        Ignored
    }
}

impl<'de> InnerListVisitor<'de> for Ignored {
    fn item(&mut self) -> impl ItemVisitor<'de> {
        Ignored
    }

    fn finish(self) -> impl ParameterVisitor<'de> {
        Ignored
    }
}

impl<'de> DictionaryVisitor<'de> for Ignored {
    type Out = ();

    fn entry(&mut self, _key: &'de KeyRef) -> impl EntryVisitor<'de> {
        Ignored
    }

    fn finish(self) -> Self::Out {}
}

impl<'de> ListVisitor<'de> for Ignored {
    type Out = ();

    fn entry(&mut self) -> impl EntryVisitor<'de> {
        Ignored
    }

    fn finish(self) -> Self::Out {}
}

impl<'de, V: BareItemVisitor<'de>> BareItemVisitor<'de> for Option<V> {
    type Out = Option<V::Out>;

    fn bare_item(self, value: BareItemFromInput<'de>) -> Self::Out {
        self.map(|visitor| visitor.bare_item(value))
    }
}

impl<'de, V: ParameterVisitor<'de>> ParameterVisitor<'de> for Option<V> {
    type Out = Option<V::Out>;

    fn parameter(&mut self, key: &'de KeyRef) -> impl BareItemVisitor<'de> {
        self.as_mut().map(|visitor| visitor.parameter(key))
    }

    fn finish(self) -> Self::Out {
        self.map(V::finish)
    }
}

impl<'de, V: EntryVisitor<'de>> EntryVisitor<'de> for Option<V> {
    fn item(self) -> impl ItemVisitor<'de> {
        self.map(V::item)
    }

    fn inner_list(self) -> impl InnerListVisitor<'de> {
        self.map(V::inner_list)
    }
}

impl<'de, V: InnerListVisitor<'de>> InnerListVisitor<'de> for Option<V> {
    fn item(&mut self) -> impl ItemVisitor<'de> {
        self.as_mut().map(V::item)
    }

    fn finish(self) -> impl ParameterVisitor<'de> {
        self.map(V::finish)
    }
}
