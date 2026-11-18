// This example demonstrates both dictionary- and visitor-based parsing of the
// Priority header according to https://httpwg.org/specs/rfc9218.html.

use std::convert::Infallible;

use sfv::{BareItemFromInput, KeyRef, visitor};

const KEY_U: &str = "u";
const KEY_I: &str = "i";

const DEFAULT_URGENCY: u8 = 3;
const DEFAULT_INCREMENTAL: bool = false;

#[derive(Debug, PartialEq)]
pub struct Priority {
    urgency: u8,
    incremental: bool,
}

impl Default for Priority {
    fn default() -> Self {
        Self {
            urgency: DEFAULT_URGENCY,
            incremental: DEFAULT_INCREMENTAL,
        }
    }
}

fn parse_urgency(v: Option<sfv::Integer>) -> u8 {
    v.and_then(|v| u8::try_from(v).ok())
        .filter(|v| *v <= 7)
        .unwrap_or(DEFAULT_URGENCY)
}

fn parse_incremental(v: Option<bool>) -> bool {
    v.unwrap_or(DEFAULT_INCREMENTAL)
}

// Notes for real-world implementations:
//
// - Accept `&[u8]` rather than `&str`, since HTTP header field values are not
//   necessarily UTF-8.
// - Encapsulate parsing in a function rather than exposing
//   `From<&sfv::Dictionary>` or visitor types publicly: RFC 9218 defines
//   `Priority` in terms of RFC 8941, whereas `sfv::Parser::new` defaults to
//   RFC 9651, so letting callers construct `sfv::Parser` themselves could lead
//   to accidentally parsing with the wrong RFC version.
// - Expose only one of the two implementations below, which trade off
//   allocations, source-code complexity, and monomorphization costs:
//   - `parse_from_dictionary` is simpler to write and avoids monomorphizing a
//     custom visitor, but requires the `parsed-types` Cargo feature (which
//     entails an additional crate dependency) and allocates an intermediate
//     `Dictionary`.
//   - `parse_with_visitor` involves more source code and monomorphization, but
//     records only the minimal state needed to parse the header (avoiding the
//     intermediate `Dictionary` allocation) and allows the `parsed-types` Cargo
//     feature to be disabled.

/// Parses a `Priority` header value via an intermediate [`sfv::Dictionary`].
///
/// # Errors
/// Returns an error if `input` is not a syntactically valid RFC 8941
/// dictionary.
#[cfg(feature = "parsed-types")]
pub fn parse_from_dictionary(input: &[u8]) -> Result<Priority, sfv::Error> {
    let dict: sfv::Dictionary = sfv::Parser::new(input)
        .with_version(sfv::Version::Rfc8941)
        .parse()?;

    Ok(Priority {
        urgency: parse_urgency(match dict.get(KEY_U) {
            Some(sfv::ListEntry::Item(sfv::Item { bare_item, .. })) => bare_item.as_integer(),
            _ => None,
        }),
        incremental: parse_incremental(match dict.get(KEY_I) {
            Some(sfv::ListEntry::Item(sfv::Item { bare_item, .. })) => bare_item.as_boolean(),
            _ => None,
        }),
    })
}

/// Parses a `Priority` header value incrementally using a visitor.
///
/// # Errors
/// Returns an error if `input` is not a syntactically valid RFC 8941
/// dictionary.
pub fn parse_with_visitor(input: &[u8]) -> Result<Priority, sfv::Error> {
    struct Visitor(Priority);

    enum Parameter<'a> {
        U(&'a mut u8),
        I(&'a mut bool),
    }

    impl<'de> visitor::DictionaryVisitor<'de> for Visitor {
        type Out = Priority;
        type Error = Infallible;

        fn entry(
            &mut self,
            key: &'de KeyRef,
        ) -> Result<impl visitor::EntryVisitor<'de>, Self::Error> {
            Ok(match key.as_str() {
                KEY_U => Some(Parameter::U(&mut self.0.urgency)),
                KEY_I => Some(Parameter::I(&mut self.0.incremental)),
                // Per https://httpwg.org/specs/rfc9218.html#parameters unknown
                // dictionary keys are ignored.
                _ => None,
            })
        }

        fn finish(self) -> Result<Self::Out, Self::Error> {
            Ok(self.0)
        }
    }

    impl<'de> visitor::ItemVisitor<'de> for Parameter<'_> {
        type Out = ();
        type Error = Infallible;

        fn bare_item(
            self,
            bare_item: BareItemFromInput<'de>,
        ) -> Result<impl visitor::ParameterVisitor<'de, Out = Self::Out>, Self::Error> {
            // Per https://httpwg.org/specs/rfc9218.html#parameters values of
            // unexpected types and out-of-range values are ignored. Since the
            // same dictionary key can appear multiple times in the input, and
            // only the last value may be considered per Structured Field
            // semantics, we overwrite any existing value with the default on
            // error.
            match self {
                Self::U(urgency) => *urgency = parse_urgency(bare_item.as_integer()),
                Self::I(incremental) => *incremental = parse_incremental(bare_item.as_boolean()),
            }
            // Neither https://httpwg.org/specs/rfc9218.html#urgency nor
            // https://httpwg.org/specs/rfc9218.html#incremental defines
            // parameters.
            Ok(visitor::Ignored)
        }
    }

    impl<'de> visitor::EntryVisitor<'de> for Parameter<'_> {
        type Error = Infallible;

        fn item(self) -> Result<impl visitor::ItemVisitor<'de>, Self::Error> {
            Ok(self)
        }

        fn inner_list(self) -> Result<impl visitor::InnerListVisitor<'de>, Self::Error> {
            // Per https://httpwg.org/specs/rfc9218.html#parameters values of
            // unexpected types are ignored. Since the same dictionary key can
            // appear multiple times in the input, and only the last value may
            // be considered per Structured Field semantics, we overwrite any
            // existing value with the default.
            match self {
                Self::U(urgency) => *urgency = DEFAULT_URGENCY,
                Self::I(incremental) => *incremental = DEFAULT_INCREMENTAL,
            }
            Ok(visitor::Ignored)
        }
    }

    sfv::Parser::new(input)
        .with_version(sfv::Version::Rfc8941)
        .parse_dictionary_with_visitor(Visitor(Priority::default()))
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), sfv::Error> {
    let valid_examples = [
        // From https://httpwg.org/specs/rfc9218.html#parameters: "When
        // receiving an HTTP request that does not carry these priority
        // parameters, a server SHOULD act as if their default values were
        // specified."
        (
            "",
            Priority {
                urgency: 3,
                incremental: false,
            },
        ),
        // https://httpwg.org/specs/rfc9218.html#incremental
        (
            "u=5, i",
            Priority {
                urgency: 5,
                incremental: true,
            },
        ),
        // Unknown key
        (
            "x;a, u=5, i",
            Priority {
                urgency: 5,
                incremental: true,
            },
        ),
        // Unexpected type for urgency
        (
            "u=(), i",
            Priority {
                urgency: 3,
                incremental: true,
            },
        ),
        // Unexpected type for urgency
        (
            "u=6.5, i",
            Priority {
                urgency: 3,
                incremental: true,
            },
        ),
        // Urgency below minimum
        (
            "u=-1, i",
            Priority {
                urgency: 3,
                incremental: true,
            },
        ),
        // Urgency above maximum
        (
            "u=8, i",
            Priority {
                urgency: 3,
                incremental: true,
            },
        ),
        // Unexpected type for incremental
        (
            "i=(), u=5",
            Priority {
                urgency: 5,
                incremental: false,
            },
        ),
        // Unexpected type for incremental
        (
            "i=1, u=5",
            Priority {
                urgency: 5,
                incremental: false,
            },
        ),
        // Parameters are ignored
        (
            "u=5;x, i;y",
            Priority {
                urgency: 5,
                incremental: true,
            },
        ),
        // When duplicate keys are encountered, only last's value is used
        (
            "u=6, i, u=5, i=?0",
            Priority {
                urgency: 5,
                incremental: false,
            },
        ),
        // When duplicate keys are encountered, only last's value is used
        (
            "u=6, i, u=(), i=()",
            Priority {
                urgency: 3,
                incremental: false,
            },
        ),
    ];

    for (input, expected) in valid_examples {
        #[cfg(feature = "parsed-types")]
        assert_eq!(
            parse_from_dictionary(input.as_bytes())?,
            expected,
            "{input}"
        );

        assert_eq!(parse_with_visitor(input.as_bytes())?, expected, "{input}");
    }

    for invalid_input in [
        // Trailing comma
        "u=5,",
        // Dates are an RFC 9651 addition and invalid in RFC 8941 / RFC 9218
        "u=5, x=@1659578233",
    ] {
        #[cfg(feature = "parsed-types")]
        assert!(
            parse_from_dictionary(invalid_input.as_bytes()).is_err(),
            "{invalid_input}"
        );

        assert!(
            parse_with_visitor(invalid_input.as_bytes()).is_err(),
            "{invalid_input}"
        );
    }

    Ok(())
}
