/*!
Contains traits for parsing structured-field values incrementally.

The various visitor methods are invoked *during* parsing, i.e. before validation
of the entire input is complete. Therefore, users of these traits must carefully
consider whether they want to induce side effects or perform expensive
operations *before* knowing whether the entire input is valid, and must
*especially* avoid breaking RFC 8941/9651 requirements around the handling of
duplicate parameter/dictionary keys. Specifically, validation of parameters and
keys typically needs to be done as part of the `finish` method of
`ParameterVisitor`, `ListVisitor`, or `DictionaryVisitor`, in order to ensure
that all but the last instance of a given dictionary or parameter key is
ignored from a semantic perspective.

For example, consider a fictitious dictionary header `Foo` defined to contain
a single required top-level key, `color`, which is either the token `blue` or
the token `red`, with an optional boolean parameter `strong`, defaulting to
`false`. If `color` or `strong` has the wrong type, the entire header is
considered invalid.

```
use std::convert::Infallible;

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
struct Foo {
    color: Color,
    strong: bool,
}

#[derive(Debug)]
enum Error {
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
    enum State {
        ColorMissing,
        ColorWrongType,
        ColorInvalid,
        StrongWrongType,
        Valid(Foo),
    }

    struct ParamVisitor<'a> {
        state: &'a mut State,
        color: Color,
    }

    impl<'de> visitor::DictionaryVisitor<'de> for State {
        type Out = Self;
        type Error = Infallible;

        fn entry(&mut self, key: &'de KeyRef) -> Result<impl visitor::EntryVisitor<'de>, Self::Error> {
            Ok(if key == KEY_COLOR {
                Some(self)
            } else {
                // Ignore other keys.
                None
            })
        }

        fn finish(self) -> Result<Self::Out, Self::Error> {
            Ok(self)
        }
    }

    impl<'de> visitor::EntryVisitor<'de> for &mut State {
        type Error = Infallible;

        fn item(self) -> Result<impl visitor::ItemVisitor<'de>, Self::Error> {
            Ok(self)
        }

        fn inner_list(self) -> Result<impl visitor::InnerListVisitor<'de>, Self::Error> {
            // It would be incorrect to return an error here: If the `color` key
            // appears more than once, doing so would cause the entire visitation to
            // fail even if the later key has the correct type. Instead, we record
            // the error in `State` and check the final state after parsing completes.
            *self = State::ColorWrongType;
            Ok(visitor::Ignored)
        }
    }

    impl<'de> visitor::ItemVisitor<'de> for &mut State {
        type Out = Option<()>;
        type Error = Infallible;

        fn bare_item(self, v: BareItemFromInput<'de>) -> Result<impl visitor::ParameterVisitor<'de, Out = Self::Out>, Self::Error> {
            // It would be incorrect to return the errors here: If the `color` key
            // appears more than once, doing so would cause the entire visitation to
            // fail even if the later key has the correct type and a valid value.
            // Instead, we record the errors in `State` and check the final state
            // after parsing completes.

            if let BareItemFromInput::Token(v) = v {
                if let Ok(color) = Color::try_from(v) {
                    *self = State::Valid(Foo { color, strong: false });
                    // Visit the parameters.
                    return Ok(Some(ParamVisitor { state: self, color }));
                }

                *self = State::ColorInvalid;
            } else {
                *self = State::ColorWrongType;
            }

            // No need to visit the parameters if the type or value is invalid.
            Ok(None)
        }
    }

    impl<'de> visitor::ParameterVisitor<'de> for ParamVisitor<'_> {
        type Out = ();
        type Error = Infallible;

        fn parameter(&mut self, key: &'de KeyRef, v: BareItemFromInput<'de>) -> Result<(), Self::Error> {
            // Ignore other parameters.
            if key == KEY_STRONG {
                // It would be incorrect to return an error here: If the `strong`
                // key appears more than once, doing so would cause the entire
                // visitation to fail even if the later key has the correct type.
                // Instead, we record the error in `State` and check the final state
                // after parsing completes.
                *self.state = if let BareItemFromInput::Boolean(strong) = v {
                    State::Valid(Foo { color: self.color, strong })
                } else {
                    State::StrongWrongType
                };
            }
            Ok(())
        }

        fn finish(self) -> Result<Self::Out, Self::Error> {
            Ok(())
        }
    }

    match sfv::Parser::new(input).parse_dictionary_with_visitor(State::ColorMissing).map_err(Error::Syntax)? {
        State::ColorMissing => Err(Error::ColorMissing),
        State::ColorWrongType => Err(Error::ColorWrongType),
        State::ColorInvalid => Err(Error::ColorInvalid),
        State::StrongWrongType => Err(Error::StrongWrongType),
        State::Valid(foo) => Ok(foo),
    }
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

// Syntax errors are also reported:
#[cfg(feature = "parsed-types")]
assert!(matches!(parse_from_dictionary(b"color=?2"), Err(Error::Syntax(_))));
assert!(matches!(parse_with_visitor(b"color=?2"), Err(Error::Syntax(_))));
```

# Returning a value from `ItemVisitor`

If a top-level item is being parsed, the visitor can return the value directly:

```
# use sfv::visitor::{Ignored, ItemVisitor, ParameterVisitor, parameter_visitor_with};
# use sfv::{BareItemFromInput, TokenRef, token_ref};
# fn main() -> Result<(), sfv::Error> {
struct Visitor;

impl<'de> ItemVisitor<'de> for Visitor {
  type Out = Option<&'de TokenRef>;
  type Error = std::convert::Infallible;

  fn bare_item(self, bare_item: BareItemFromInput<'de>) -> Result<impl ParameterVisitor<'de, Out = Self::Out>, Self::Error> {
      Ok(parameter_visitor_with(Ignored, move |_| {
        Ok(if let BareItemFromInput::Token(token) = bare_item {
            Some(token)
        } else {
            None
        })
     }))
  }
}

assert_eq!(
  Some(token_ref("abc")),
  sfv::Parser::new("abc").parse_item_with_visitor(Visitor)?,
);
# Ok(())
# }
```

Or without the `Option` at all:

```
# use sfv::visitor::{Ignored, ItemVisitor, ParameterVisitor, parameter_visitor_with};
# use sfv::{BareItemFromInput, TokenRef, token_ref};
# fn main() -> Result<(), sfv::Error> {
struct Visitor;

#[derive(Debug)]
struct ExpectedToken;

impl std::fmt::Display for ExpectedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("expected token")
    }
}

impl std::error::Error for ExpectedToken {}

impl<'de> ItemVisitor<'de> for Visitor {
  type Out = &'de TokenRef;
  type Error = ExpectedToken;

  fn bare_item(self, bare_item: BareItemFromInput<'de>) -> Result<impl ParameterVisitor<'de, Out = Self::Out>, Self::Error> {
      if let BareItemFromInput::Token(token) = bare_item {
          Ok(parameter_visitor_with(Ignored, move |_| Ok(token)))
      } else {
          Err(ExpectedToken)
      }
  }
}

assert_eq!(
  token_ref("abc"),
  sfv::Parser::new("abc").parse_item_with_visitor(Visitor)?,
);

assert!(sfv::Parser::new("123").parse_item_with_visitor(Visitor).is_err());
# Ok(())
# }
```

Or using a function, given the blanket implementation of [`ItemVisitor`] for [`FnOnce`]:

```
# use sfv::visitor::{Ignored, ParameterVisitor, parameter_visitor_with};
# use sfv::{BareItemFromInput, TokenRef, token_ref};
# fn main() -> Result<(), sfv::Error> {
# #[derive(Debug)]
# struct ExpectedToken;
#
# impl std::fmt::Display for ExpectedToken {
#    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
#        f.write_str("expected token")
#    }
# }
#
# impl std::error::Error for ExpectedToken {}
#
fn as_token<'de>(bare_item: BareItemFromInput<'de>) -> Result<impl ParameterVisitor<'de, Out = &'de TokenRef>, ExpectedToken> {
  if let BareItemFromInput::Token(token) = bare_item {
      Ok(parameter_visitor_with(Ignored, move |_| Ok(token)))
  } else {
      Err(ExpectedToken)
  }
}

assert_eq!(
  token_ref("abc"),
  sfv::Parser::new("abc").parse_item_with_visitor(as_token)?,
);
# Ok(())
# }
```

Or even:

```
# use sfv::{TokenRef, token_ref};
# fn main() -> Result<(), sfv::Error> {
assert_eq!(
  token_ref("abc"),
  sfv::Parser::new("abc").parse_item::<&TokenRef>()?,
);
# Ok(())
# }
```

# Discarding irrelevant parts

Two kinds of helpers are provided for silently discarding structured-field
parts:

- [`Ignored`]: This type implements all of the visitor traits as no-ops, and can
  be used when a visitor implementation would unconditionally do nothing. An
  example of this is when an item's bare item needs to be validated, but its
  parameters do not (e.g. because the relevant field definition prescribes
  none and permits unknown ones).

- Blanket implementations of [`ParameterVisitor`], [`ItemVisitor`],
  [`EntryVisitor`], and [`InnerListVisitor`] for [`Option<V>`] where `V`
  implements that trait: These implementations act like `Ignored` when `self` is
  [`None`], and forward to `V`'s implementation when `self` is [`Some`]. These
  can be used when the visitor dynamically handles or ignores field parts. An
  example of this is when a field definition prescribes the format of certain
  dictionary keys, but ignores unknown ones.

Note that the discarded parts are still validated during parsing: syntactic
errors in the input still cause parsing to fail even when these helpers are
used, [as required by RFC 9651](https://httpwg.org/specs/rfc9651.html#strict).

The `Foo` header example above demonstrates usage of both kinds of helpers.
*/

use std::{convert::Infallible, error::Error};

use crate::{BareItemFromInput, KeyRef};

/// A visitor whose methods are called during parameter parsing.
///
/// The lifetime `'de` is the lifetime of the input.
pub trait ParameterVisitor<'de> {
    /// The successful return type of the [`ParameterVisitor::finish`] method.
    ///
    /// Many implementations will set this to `()`. See
    /// [the module documentation](crate::visitor#returning-a-value-from-itemvisitor)
    /// for an example that does not.
    type Out;

    /// The error type that can be returned if some error occurs during parsing.
    type Error: Error;

    /// Called after a parameter has been parsed.
    ///
    /// Parsing will be terminated early if an error is returned.
    ///
    /// Note: Per [RFC 9651], when duplicate parameter keys are encountered in
    /// the same scope, all but the last instance are ignored. Implementations
    /// of this trait must respect that requirement in order to comply with the
    /// specification. For example, if parameters are stored in a map, earlier
    /// values for a given parameter key must be overwritten by later ones.
    ///
    /// [RFC 9651]: <https://httpwg.org/specs/rfc9651.html#parse-param>
    ///
    /// # Errors
    /// The error result should report the reason for any failed validation.
    fn parameter(
        &mut self,
        key: &'de KeyRef,
        value: BareItemFromInput<'de>,
    ) -> Result<(), Self::Error>;

    /// Called after all parameters have been parsed.
    ///
    /// Parsing will be terminated early if an error is returned.
    ///
    /// # Errors
    /// The error result should report the reason for any failed validation.
    fn finish(self) -> Result<Self::Out, Self::Error>;
}

/// A visitor whose methods are called during item parsing.
///
/// The lifetime `'de` is the lifetime of the input.
///
/// Use this trait with
/// [`Parser::parse_item_with_visitor`][crate::Parser::parse_item_with_visitor].
pub trait ItemVisitor<'de> {
    /// The successful return type of the returned [`ParameterVisitor::finish`]
    /// method.
    ///
    /// Many implementations will set this to `()`. See
    /// [the module documentation](crate::visitor#returning-a-value-from-itemvisitor)
    /// for an example that does not.
    type Out;

    /// The error type that can be returned if some error occurs during parsing.
    type Error: Error;

    /// Called after a bare item has been parsed.
    ///
    /// The returned visitor is used to handle the bare item's parameters.
    /// See [the module documentation](crate::visitor#discarding-irrelevant-parts)
    /// for guidance on discarding parameters.
    ///
    /// Parsing will be terminated early if an error is returned.
    ///
    /// # Errors
    /// The error result should report the reason for any failed validation.
    fn bare_item(
        self,
        bare_item: BareItemFromInput<'de>,
    ) -> Result<impl ParameterVisitor<'de, Out = Self::Out>, Self::Error>;
}

impl<'de, F, V, E> ItemVisitor<'de> for F
where
    F: FnOnce(BareItemFromInput<'de>) -> Result<V, E>,
    V: ParameterVisitor<'de>,
    E: Error,
{
    type Out = V::Out;
    type Error = E;

    fn bare_item(
        self,
        bare_item: BareItemFromInput<'de>,
    ) -> Result<impl ParameterVisitor<'de, Out = Self::Out>, Self::Error> {
        self(bare_item)
    }
}

/// A visitor whose methods are called during inner-list parsing.
///
/// The lifetime `'de` is the lifetime of the input.
pub trait InnerListVisitor<'de> {
    /// The error type that can be returned if some error occurs during parsing.
    type Error: Error;

    /// Called before an item has been parsed.
    ///
    /// The returned visitor is used to handle the bare item.
    ///
    /// Parsing will be terminated early if an error is returned.
    ///
    /// # Errors
    /// The error result should report the reason for any failed validation.
    fn item(&mut self) -> Result<impl ItemVisitor<'de>, Self::Error>;

    /// Called after all inner-list items have been parsed.
    ///
    /// The returned visitor is used to handle the inner list's parameters.
    /// See [the module documentation](crate::visitor#discarding-irrelevant-parts)
    /// for guidance on discarding parameters.
    ///
    /// Parsing will be terminated early if an error is returned.
    ///
    /// # Errors
    /// The error result should report the reason for any failed validation.
    fn finish(self) -> Result<impl ParameterVisitor<'de>, Self::Error>;
}

/// A visitor whose methods are called during entry parsing.
///
/// The lifetime `'de` is the lifetime of the input.
pub trait EntryVisitor<'de> {
    /// The error type that can be returned if some error occurs during parsing.
    type Error: Error;

    /// Called before an item has been parsed.
    ///
    /// The returned visitor is used to handle the item.
    ///
    /// Parsing will be terminated early if an error is returned.
    ///
    /// # Errors
    /// The error result should report the reason for any failed validation.
    fn item(self) -> Result<impl ItemVisitor<'de>, Self::Error>;

    /// Called before an inner list has been parsed.
    ///
    /// The returned visitor is used to handle the inner list.
    ///
    /// Parsing will be terminated early if an error is returned.
    ///
    /// # Errors
    /// The error result should report the reason for any failed validation.
    fn inner_list(self) -> Result<impl InnerListVisitor<'de>, Self::Error>;
}

/// A visitor whose methods are called during dictionary parsing.
///
/// The lifetime `'de` is the lifetime of the input.
///
/// Use this trait with
/// [`Parser::parse_dictionary_with_visitor`][crate::Parser::parse_dictionary_with_visitor].
pub trait DictionaryVisitor<'de> {
    /// The successful return type of the [`DictionaryVisitor::finish`] method.
    type Out;

    /// The error type that can be returned if some error occurs during parsing.
    type Error: Error;

    /// Called after a dictionary key has been parsed.
    ///
    /// The returned visitor is used to handle the associated value.
    /// See [the module documentation](crate::visitor#discarding-irrelevant-parts)
    /// for guidance on discarding entries.
    ///
    /// Parsing will be terminated early if an error is returned.
    ///
    /// Note: Per [RFC 9651], when duplicate dictionary keys are encountered in
    /// the same scope, all but the last instance are ignored. Implementations
    /// of this trait must respect that requirement in order to comply with the
    /// specification. For example, if dictionary entries are stored in a map,
    /// earlier values for a given dictionary key must be overwritten by later
    /// ones.
    ///
    /// [RFC 9651]: <https://httpwg.org/specs/rfc9651.html#parse-dictionary>
    ///
    /// # Errors
    /// The error result should report the reason for any failed validation.
    fn entry(&mut self, key: &'de KeyRef) -> Result<impl EntryVisitor<'de>, Self::Error>;

    /// Called after all dictionary keys have been parsed.
    ///
    /// Parsing will be terminated early if an error is returned.
    ///
    /// # Errors
    /// The error result should report the reason for any failed validation.
    fn finish(self) -> Result<Self::Out, Self::Error>;
}

/// A visitor whose methods are called during list parsing.
///
/// The lifetime `'de` is the lifetime of the input.
///
/// Use this trait with
/// [`Parser::parse_list_with_visitor`][crate::Parser::parse_list_with_visitor].
pub trait ListVisitor<'de> {
    /// The successful return type of the [`ListVisitor::finish`] method.
    type Out;

    /// The error type that can be returned if some error occurs during parsing.
    type Error: Error;

    /// Called before a list entry has been parsed.
    ///
    /// The returned visitor is used to handle the entry.
    ///
    /// Parsing will be terminated early if an error is returned.
    ///
    /// # Errors
    /// The error result should report the reason for any failed validation.
    fn entry(&mut self) -> Result<impl EntryVisitor<'de>, Self::Error>;

    /// Called after all list entries have been parsed.
    ///
    /// Parsing will be terminated early if an error is returned.
    ///
    /// # Errors
    /// The error result should report the reason for any failed validation.
    fn finish(self) -> Result<Self::Out, Self::Error>;
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

impl<'de> ParameterVisitor<'de> for Ignored {
    type Out = ();
    type Error = Infallible;

    fn parameter(
        &mut self,
        _key: &'de KeyRef,
        _value: BareItemFromInput<'de>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn finish(self) -> Result<Self::Out, Self::Error> {
        Ok(())
    }
}

impl<'de> ItemVisitor<'de> for Ignored {
    type Out = ();
    type Error = Infallible;

    fn bare_item(
        self,
        _bare_item: BareItemFromInput<'de>,
    ) -> Result<impl ParameterVisitor<'de, Out = Self::Out>, Self::Error> {
        Ok(Ignored)
    }
}

impl<'de> EntryVisitor<'de> for Ignored {
    type Error = Infallible;

    fn item(self) -> Result<impl ItemVisitor<'de>, Self::Error> {
        Ok(Ignored)
    }

    fn inner_list(self) -> Result<impl InnerListVisitor<'de>, Self::Error> {
        Ok(Ignored)
    }
}

impl<'de> InnerListVisitor<'de> for Ignored {
    type Error = Infallible;

    fn item(&mut self) -> Result<impl ItemVisitor<'de>, Self::Error> {
        Ok(Ignored)
    }

    fn finish(self) -> Result<impl ParameterVisitor<'de>, Self::Error> {
        Ok(Ignored)
    }
}

impl<'de> DictionaryVisitor<'de> for Ignored {
    type Out = ();
    type Error = Infallible;

    fn entry(&mut self, _key: &'de KeyRef) -> Result<impl EntryVisitor<'de>, Self::Error> {
        Ok(Ignored)
    }

    fn finish(self) -> Result<Self::Out, Self::Error> {
        Ok(())
    }
}

impl<'de> ListVisitor<'de> for Ignored {
    type Out = ();
    type Error = Infallible;

    fn entry(&mut self) -> Result<impl EntryVisitor<'de>, Self::Error> {
        Ok(Ignored)
    }

    fn finish(self) -> Result<Self::Out, Self::Error> {
        Ok(())
    }
}

fn map_visitor<V, T, E>(
    visitor: Option<V>,
    f: impl FnOnce(V) -> Result<T, E>,
) -> Result<Option<T>, E> {
    match visitor {
        None => Ok(None),
        Some(visitor) => f(visitor).map(Some),
    }
}

impl<'de, V: ParameterVisitor<'de>> ParameterVisitor<'de> for Option<V> {
    type Out = Option<V::Out>;
    type Error = V::Error;

    fn parameter(
        &mut self,
        key: &'de KeyRef,
        value: BareItemFromInput<'de>,
    ) -> Result<(), Self::Error> {
        match *self {
            None => Ok(()),
            Some(ref mut visitor) => visitor.parameter(key, value),
        }
    }

    fn finish(self) -> Result<Self::Out, Self::Error> {
        map_visitor(self, V::finish)
    }
}

impl<'de, V: ItemVisitor<'de>> ItemVisitor<'de> for Option<V> {
    type Out = Option<V::Out>;
    type Error = V::Error;

    fn bare_item(
        self,
        bare_item: BareItemFromInput<'de>,
    ) -> Result<impl ParameterVisitor<'de, Out = Self::Out>, Self::Error> {
        map_visitor(self, |visitor| visitor.bare_item(bare_item))
    }
}

impl<'de, V: EntryVisitor<'de>> EntryVisitor<'de> for Option<V> {
    type Error = V::Error;

    fn item(self) -> Result<impl ItemVisitor<'de>, Self::Error> {
        map_visitor(self, V::item)
    }

    fn inner_list(self) -> Result<impl InnerListVisitor<'de>, Self::Error> {
        map_visitor(self, V::inner_list)
    }
}

impl<'de, V: InnerListVisitor<'de>> InnerListVisitor<'de> for Option<V> {
    type Error = V::Error;

    fn item(&mut self) -> Result<impl ItemVisitor<'de>, Self::Error> {
        map_visitor(self.as_mut(), V::item)
    }

    fn finish(self) -> Result<impl ParameterVisitor<'de>, Self::Error> {
        map_visitor(self, V::finish)
    }
}

/// A visitor that cannot be instantiated, but can be used as a type in
/// situations guaranteed to return an error `Result`, analogous to
/// [`std::convert::Infallible`].
///
/// When [`!`] is stabilized, this type will be replaced with an alias for it.
#[derive(Clone, Copy, Debug)]
pub enum Never {}

impl<'de> ParameterVisitor<'de> for Never {
    type Out = ();
    type Error = Infallible;

    fn parameter(
        &mut self,
        _key: &'de KeyRef,
        _value: BareItemFromInput<'de>,
    ) -> Result<(), Self::Error> {
        match *self {}
    }

    fn finish(self) -> Result<Self::Out, Self::Error> {
        match self {}
    }
}

impl<'de> ItemVisitor<'de> for Never {
    type Out = ();
    type Error = Infallible;

    fn bare_item(
        self,
        _bare_item: BareItemFromInput<'de>,
    ) -> Result<impl ParameterVisitor<'de, Out = Self::Out>, Self::Error> {
        Ok(self)
    }
}

impl<'de> EntryVisitor<'de> for Never {
    type Error = Infallible;

    fn item(self) -> Result<impl ItemVisitor<'de>, Self::Error> {
        Ok(self)
    }

    fn inner_list(self) -> Result<impl InnerListVisitor<'de>, Self::Error> {
        Ok(self)
    }
}

impl<'de> InnerListVisitor<'de> for Never {
    type Error = Infallible;

    fn item(&mut self) -> Result<impl ItemVisitor<'de>, Self::Error> {
        Ok(*self)
    }

    fn finish(self) -> Result<impl ParameterVisitor<'de>, Self::Error> {
        Ok(self)
    }
}

/// Returns a `ParameterVisitor` that delegates to another visitor but invokes
/// a function to return its value.
///
/// The returned visitor behaves as follows:
///
/// - [`ParameterVisitor::parameter`] forwards directly to `visitor.parameter`
/// - [`ParameterVisitor::finish`] returns `finish(visitor.finish()?)`
///
/// This can be used to propagate a value of type `T` produced within an
/// [`ItemVisitor::bare_item`] call to a parameter visitor `V` that will return
/// it, and even delay an expensive operation producing `T` until the parameters
/// have been parsed successfully.
///
/// See [the module documentation](crate::visitor#returning-a-value-from-itemvisitor)
/// for an example.
pub fn parameter_visitor_with<'de, V, T>(
    visitor: V,
    finish: impl FnOnce(V::Out) -> Result<T, V::Error>,
) -> impl ParameterVisitor<'de, Out = T, Error = V::Error>
where
    V: ParameterVisitor<'de>,
{
    ParameterVisitorWith { visitor, finish }
}

struct ParameterVisitorWith<V, F> {
    visitor: V,
    finish: F,
}

impl<'de, V, F, T> ParameterVisitor<'de> for ParameterVisitorWith<V, F>
where
    V: ParameterVisitor<'de>,
    F: FnOnce(V::Out) -> Result<T, V::Error>,
{
    type Out = T;
    type Error = V::Error;

    fn parameter(
        &mut self,
        key: &'de KeyRef,
        value: BareItemFromInput<'de>,
    ) -> Result<(), Self::Error> {
        self.visitor.parameter(key, value)
    }

    fn finish(self) -> Result<Self::Out, Self::Error> {
        (self.finish)(self.visitor.finish()?)
    }
}

/// A type that can be produced from an [`ItemVisitor`].
///
/// Use this with [`crate::Parser::parse_item`].
pub trait MakeItemVisitor<'de> {
    /// Returns an item visitor that produces `Self` on success.
    fn make_item_visitor() -> impl ItemVisitor<'de, Out = Self>;
}

/// A type that can be produced from a [`ListVisitor`].
///
/// Use this with [`crate::Parser::parse_list`].
pub trait MakeListVisitor<'de> {
    /// Returns a list visitor that produces `Self` on success.
    fn make_list_visitor() -> impl ListVisitor<'de, Out = Self>;
}

/// A type that can be produced from a [`DictionaryVisitor`].
///
/// Use this with [`crate::Parser::parse_dictionary`].
pub trait MakeDictionaryVisitor<'de> {
    /// Returns a dictionary visitor that produces `Self` on success.
    fn make_dictionary_visitor() -> impl DictionaryVisitor<'de, Out = Self>;
}

impl<'de, V> MakeItemVisitor<'de> for V
where
    V: ItemVisitor<'de, Out = Self> + Default,
{
    fn make_item_visitor() -> impl ItemVisitor<'de, Out = Self> {
        V::default()
    }
}

impl<'de, V> MakeListVisitor<'de> for V
where
    V: ListVisitor<'de, Out = Self> + Default,
{
    fn make_list_visitor() -> impl ListVisitor<'de, Out = Self> {
        V::default()
    }
}

impl<'de, V> MakeDictionaryVisitor<'de> for V
where
    V: DictionaryVisitor<'de, Out = Self> + Default,
{
    fn make_dictionary_visitor() -> impl DictionaryVisitor<'de, Out = Self> {
        V::default()
    }
}

#[allow(clippy::unnecessary_wraps)]
fn infallible_bare_item_visitor<'de, T>(
    bare_item: BareItemFromInput<'de>,
) -> Result<impl ParameterVisitor<'de, Out = T>, Infallible>
where
    T: From<BareItemFromInput<'de>>,
{
    Ok(parameter_visitor_with(Ignored, |()| Ok(T::from(bare_item))))
}

/// Makes an item visitor expecting any bare item and ignoring parameters.
impl<'de> MakeItemVisitor<'de> for BareItemFromInput<'de> {
    fn make_item_visitor() -> impl ItemVisitor<'de, Out = Self> {
        infallible_bare_item_visitor
    }
}

/// Makes an item visitor expecting any bare item and ignoring parameters.
impl<'de> MakeItemVisitor<'de> for super::BareItem {
    fn make_item_visitor() -> impl ItemVisitor<'de, Out = Self> {
        infallible_bare_item_visitor
    }
}

#[derive(Debug)]
enum BareItemType {
    Decimal,
    Integer,
    String,
    ByteSequence,
    Boolean,
    Token,
    Date,
    DisplayString,
}

impl BareItemFromInput<'_> {
    fn ty(&self) -> BareItemType {
        match *self {
            Self::Decimal(_) => BareItemType::Decimal,
            Self::Integer(_) => BareItemType::Integer,
            Self::String(_) => BareItemType::String,
            Self::ByteSequence(_) => BareItemType::ByteSequence,
            Self::Boolean(_) => BareItemType::Boolean,
            Self::Token(_) => BareItemType::Token,
            Self::Date(_) => BareItemType::Date,
            Self::DisplayString(_) => BareItemType::DisplayString,
        }
    }
}

impl std::fmt::Display for BareItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match *self {
            Self::Decimal => "decimal",
            Self::Integer => "integer",
            Self::String => "string",
            Self::ByteSequence => "byte sequence",
            Self::Boolean => "boolean",
            Self::Token => "token",
            Self::Date => "date",
            Self::DisplayString => "display string",
        })
    }
}

#[derive(Debug)]
struct UnexpectedBareItemType {
    expected: BareItemType,
    got: BareItemType,
}

impl std::fmt::Display for UnexpectedBareItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "unexpected bare item type: expected {}, got {}",
            self.expected, self.got
        )
    }
}

impl Error for UnexpectedBareItemType {}

macro_rules! impl_make_item_visitor_ignoring_params {
    ($($var: ident($t: ty) $doc: literal,)+) => {
        $(
            /// Makes an item visitor expecting
            #[doc = $doc]
            /// bare item and ignoring parameters.
            impl<'de> MakeItemVisitor<'de> for $t {
                fn make_item_visitor() -> impl ItemVisitor<'de, Out = Self> {
                    |v| {
                        if let BareItemFromInput::$var(v) = v {
                            Ok(parameter_visitor_with(Ignored, move |()| Ok(v)))
                        } else {
                            Err(UnexpectedBareItemType {
                                expected: BareItemType::$var,
                                got: v.ty(),
                            })
                        }
                    }
                }
            }
        )+
    };
}

impl_make_item_visitor_ignoring_params! {
    Decimal(super::Decimal) "a decimal",
    Integer(super::Integer) "an integer",
    String(std::borrow::Cow<'de, super::StringRef>) "a string",
    ByteSequence(Vec<u8>) "a byte sequence",
    Boolean(bool) "a boolean",
    Token(&'de super::TokenRef) "a token",
    Date(super::Date) "a date",
    // deliberately omitted: DisplayString(Cow<'de, str>) "display string",
}
