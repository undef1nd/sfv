/*!
`sfv` is an implementation of *Structured Field Values for HTTP*, as specified in [RFC 8941](https://httpwg.org/specs/rfc8941.html) for parsing and serializing HTTP field values.
It also exposes a set of types that might be useful for defining new structured fields.

# Data Structures

There are three types of structured fields:

- `Item` -- an `Integer`, `Decimal`, `String`, `Token`, `Byte Sequence`, or `Boolean`. It can have associated `Parameters`.
- `List` -- an array of zero or more members, each of which can be an `Item` or an `InnerList`, both of which can have `Parameters`.
- `Dictionary` -- an ordered map of name-value pairs, where the names are short textual strings and the values are `Item`s or arrays of `Items` (represented with `InnerList`), both of which can have associated parameters. There can be zero or more members, and their names are unique in the scope of the `Dictionary` they occur within.

There are also a few lower-level types used to construct structured field values:
- `BareItem` is used as `Item`'s value or as a parameter value in `Parameters`.
- `Parameters` are an ordered map of key-value pairs that are associated with an `Item` or `InnerList`. The keys are unique within the scope the `Parameters` they occur within, and the values are `BareItem`.
- `InnerList` is an array of zero or more `Items`. Can have associated `Parameters`.
- `ListEntry` represents either `Item` or `InnerList` as a member of `List` or as member-value in `Dictionary`.

# Examples

*/
#![cfg_attr(
    feature = "parsed-types",
    doc = r##"
### Parsing

```
use sfv::Parser;
# fn main() -> Result<(), sfv::Error> {
// Parsing a structured field value of Item type.
let input = "12.445;foo=bar";
let item = Parser::from_str(input).parse_item()?;
println!("{:#?}", item);

// Parsing a structured field value of List type.
let input = r#"1;a=tok, ("foo" "bar");baz, ()"#;
let list = Parser::from_str(input).parse_list()?;
println!("{:#?}", list);

// Parsing a structured field value of Dictionary type.
let input = "a=?0, b, c; foo=bar, rating=1.5, fruits=(apple pear)";
let dict = Parser::from_str(input).parse_dictionary()?;
println!("{:#?}", dict);
# Ok(())
# }
```

### Getting Parsed Value Members
```
use sfv::*;
# fn main() -> Result<(), sfv::Error> {
let input = "u=2, n=(* foo 2)";
let dict = Parser::from_str(input).parse_dictionary()?;

match dict.get("u") {
    Some(ListEntry::Item(item)) => match &item.bare_item {
        BareItem::Token(val) => { /* ... */ }
        BareItem::Integer(val) => { /* ... */ }
        BareItem::Boolean(val) => { /* ... */ }
        BareItem::Decimal(val) => { /* ... */ }
        BareItem::String(val) => { /* ... */ }
        BareItem::ByteSeq(val) => { /* ... */ }
    },
    Some(ListEntry::InnerList(inner_list)) => { /* ... */ }
    None => { /* ... */ }
}
# Ok(())
# }
```
"##
)]
/*!
### Serialization
Serializes an `Item`:
```
use sfv::{Decimal, ItemSerializer, KeyRef, StringRef};
use std::convert::TryFrom;

# fn main() -> Result<(), sfv::Error> {
let serialized_item = ItemSerializer::new()
    .bare_item(StringRef::from_str("foo")?)
    .parameter(KeyRef::from_str("key")?, Decimal::try_from(13.45655)?)
    .finish();

assert_eq!(serialized_item, r#""foo";key=13.457"#);
# Ok(())
# }
```

Serializes a `List`:
```
use sfv::{KeyRef, ListSerializer, StringRef, TokenRef};

# fn main() -> Result<(), sfv::Error> {
let mut ser = ListSerializer::new();

ser.bare_item(TokenRef::from_str("tok")?);

{
    let mut ser = ser.inner_list();

    ser.bare_item(99).parameter(KeyRef::from_str("key")?, false);

    ser.bare_item(StringRef::from_str("foo")?);

    ser.finish().parameter(KeyRef::from_str("bar")?, true);
}

let serialized_list = ser.finish()?;

assert_eq!(
    serialized_list,
    r#"tok, (99;key=?0 "foo");bar"#
);
# Ok(())
# }
```

Serializes a `Dictionary`:
```
use sfv::{DictSerializer, KeyRef, StringRef};

# fn main() -> Result<(), sfv::Error> {
let mut ser = DictSerializer::new();

ser.bare_item(KeyRef::from_str("key1")?, StringRef::from_str("apple")?);

ser.bare_item(KeyRef::from_str("key2")?, true);

ser.bare_item(KeyRef::from_str("key3")?, false);

let serialized_dict = ser.finish()?;

assert_eq!(
    serialized_dict,
    r#"key1="apple", key2, key3=?0"#
);
# Ok(())
# }
```

# Crate features

- `parsed-types` (enabled by default) -- When enabled, exposes fully owned types
  `Item`, `Dictionary`, `List`, and their components, which can be obtained from
  [`Parser::parse_item`], etc. These types are implemented using the
  [`indexmap`](https://crates.io/crates/indexmap) crate, so disabling this
  feature can avoid that dependency if parsing using a visitor
  ([`Parser::parse_item_with_visitor`], etc.) is sufficient.

- `arbitrary` -- Implements the
  [`Arbitrary`](https://docs.rs/arbitrary/1.4.1/arbitrary/trait.Arbitrary.html)
  trait for this crate's types, making them easier to use with fuzzing.
*/

mod decimal;
mod error;
mod integer;
mod key;
#[cfg(feature = "parsed-types")]
mod parsed;
mod parser;
mod ref_serializer;
mod serializer;
mod string;
mod token;
mod utils;
pub mod visitor;

#[cfg(test)]
mod test_decimal;
#[cfg(test)]
mod test_integer;
#[cfg(test)]
mod test_key;
#[cfg(test)]
mod test_parser;
#[cfg(test)]
mod test_serializer;
#[cfg(test)]
mod test_string;
#[cfg(test)]
mod test_token;

use std::borrow::{Borrow, Cow};
use std::convert::TryFrom;

pub use decimal::Decimal;
pub use error::Error;
pub use integer::{integer, Integer};
pub use key::{key_ref, Key, KeyRef};
pub use parser::Parser;
pub use ref_serializer::{
    DictSerializer, InnerListSerializer, ItemSerializer, ListSerializer, ParameterSerializer,
};
pub use string::{string_ref, String, StringRef};
pub use token::{token_ref, Token, TokenRef};

#[cfg(feature = "parsed-types")]
pub use parsed::{Dictionary, InnerList, Item, List, ListEntry, Parameters};

#[cfg(feature = "parsed-types")]
pub use serializer::SerializeValue;

type SFVResult<T> = std::result::Result<T, Error>;

/// An abstraction over multiple kinds of ownership of a bare item.
///
/// In general most users will be interested in:
/// - [`BareItem`], for completely owned data
/// - [`RefBareItem`], for completely borrowed data
/// - [`BareItemFromInput`], for data borrowed from input when possible
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum GenericBareItem<S, B, T> {
    // sf-decimal  = ["-"] 1*12DIGIT "." 1*3DIGIT
    Decimal(Decimal),
    // sf-integer = ["-"] 1*15DIGIT
    Integer(Integer),
    // sf-string = DQUOTE *chr DQUOTE
    // chr       = unescaped / escaped
    // unescaped = %x20-21 / %x23-5B / %x5D-7E
    // escaped   = "\" ( DQUOTE / "\" )
    String(S),
    // ":" *(base64) ":"
    // base64    = ALPHA / DIGIT / "+" / "/" / "="
    ByteSeq(B),
    // sf-boolean = "?" boolean
    // boolean    = "0" / "1"
    Boolean(bool),
    // sf-token = ( ALPHA / "*" ) *( tchar / ":" / "/" )
    Token(T),
}

impl<S, B, T> GenericBareItem<S, B, T> {
    /// If the bare item is a decimal, returns it; otherwise returns `None`.
    pub fn as_decimal(&self) -> Option<Decimal> {
        match *self {
            Self::Decimal(val) => Some(val),
            _ => None,
        }
    }

    /// If the bare item is an integer, returns it; otherwise returns `None`.
    pub fn as_int(&self) -> Option<Integer> {
        match *self {
            Self::Integer(val) => Some(val),
            _ => None,
        }
    }

    /// If the bare item is a string, returns a reference to it; otherwise returns `None`.
    pub fn as_str(&self) -> Option<&S> {
        match *self {
            Self::String(ref val) => Some(val),
            _ => None,
        }
    }

    /// If the bare item is a byte sequence, returns a reference to it; otherwise returns `None`.
    pub fn as_byte_seq(&self) -> Option<&B> {
        match *self {
            Self::ByteSeq(ref val) => Some(val),
            _ => None,
        }
    }

    /// If the bare item is a boolean, returns it; otherwise returns `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Boolean(val) => Some(val),
            _ => None,
        }
    }

    /// If the bare item is a token, returns a reference to it; otherwise returns `None`.
    pub fn as_token(&self) -> Option<&T> {
        match *self {
            Self::Token(ref val) => Some(val),
            _ => None,
        }
    }
}

impl<S, B, T> From<Integer> for GenericBareItem<S, B, T> {
    fn from(val: Integer) -> Self {
        Self::Integer(val)
    }
}

impl<S, B, T> From<bool> for GenericBareItem<S, B, T> {
    fn from(val: bool) -> Self {
        Self::Boolean(val)
    }
}

impl<S, B, T> From<Decimal> for GenericBareItem<S, B, T> {
    fn from(val: Decimal) -> Self {
        Self::Decimal(val)
    }
}

impl<S, B, T> TryFrom<f32> for GenericBareItem<S, B, T> {
    type Error = Error;

    fn try_from(val: f32) -> Result<Self, Error> {
        Decimal::try_from(val).map(Self::Decimal)
    }
}

impl<S, B, T> TryFrom<f64> for GenericBareItem<S, B, T> {
    type Error = Error;

    fn try_from(val: f64) -> Result<Self, Error> {
        Decimal::try_from(val).map(Self::Decimal)
    }
}

impl<S, T> From<Vec<u8>> for GenericBareItem<S, Vec<u8>, T> {
    fn from(val: Vec<u8>) -> Self {
        Self::ByteSeq(val)
    }
}

impl From<Token> for BareItem {
    fn from(val: Token) -> BareItem {
        BareItem::Token(val)
    }
}

impl From<String> for BareItem {
    fn from(val: String) -> BareItem {
        BareItem::String(val)
    }
}

impl<'a> From<&'a [u8]> for BareItem {
    fn from(val: &'a [u8]) -> BareItem {
        BareItem::ByteSeq(val.to_owned())
    }
}

impl<'a> From<&'a TokenRef> for BareItem {
    fn from(val: &'a TokenRef) -> BareItem {
        BareItem::Token(val.to_owned())
    }
}

impl<'a> From<&'a StringRef> for BareItem {
    fn from(val: &'a StringRef) -> BareItem {
        BareItem::String(val.to_owned())
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Num {
    Decimal(Decimal),
    Integer(Integer),
}

/// A [bare item] that owns its data.
///
/// [bare item]: <https://httpwg.org/specs/rfc8941.html#item>
#[cfg_attr(
    feature = "parsed-types",
    doc = "Used to construct an [`Item`] or [`Parameters`] values."
)]
pub type BareItem = GenericBareItem<String, Vec<u8>, Token>;

/// A [bare item] that borrows its data.
///
/// Used to serialize values via [`ItemSerializer`], [`ListSerializer`], and [`DictSerializer`].
///
/// [bare item]: <https://httpwg.org/specs/rfc8941.html#item>
pub type RefBareItem<'a> = GenericBareItem<&'a StringRef, &'a [u8], &'a TokenRef>;

/// A [bare item] that borrows data from input when possible.
///
/// Used to parse input incrementally in the [`visitor`] module.
///
/// [bare item]: <https://httpwg.org/specs/rfc8941.html#item>
pub type BareItemFromInput<'a> = GenericBareItem<Cow<'a, StringRef>, Vec<u8>, &'a TokenRef>;

impl<'a, S, B, T> From<&'a GenericBareItem<S, B, T>> for RefBareItem<'a>
where
    S: Borrow<StringRef>,
    B: Borrow<[u8]>,
    T: Borrow<TokenRef>,
{
    fn from(val: &'a GenericBareItem<S, B, T>) -> RefBareItem<'a> {
        match val {
            GenericBareItem::Integer(val) => RefBareItem::Integer(*val),
            GenericBareItem::Decimal(val) => RefBareItem::Decimal(*val),
            GenericBareItem::String(val) => RefBareItem::String(val.borrow()),
            GenericBareItem::ByteSeq(val) => RefBareItem::ByteSeq(val.borrow()),
            GenericBareItem::Boolean(val) => RefBareItem::Boolean(*val),
            GenericBareItem::Token(val) => RefBareItem::Token(val.borrow()),
        }
    }
}

impl<'a> From<BareItemFromInput<'a>> for BareItem {
    fn from(val: BareItemFromInput<'a>) -> BareItem {
        match val {
            BareItemFromInput::Integer(val) => BareItem::Integer(val),
            BareItemFromInput::Decimal(val) => BareItem::Decimal(val),
            BareItemFromInput::String(val) => BareItem::String(val.into_owned()),
            BareItemFromInput::ByteSeq(val) => BareItem::ByteSeq(val),
            BareItemFromInput::Boolean(val) => BareItem::Boolean(val),
            BareItemFromInput::Token(val) => BareItem::Token(val.to_owned()),
        }
    }
}

impl<'a> From<&'a [u8]> for RefBareItem<'a> {
    fn from(val: &'a [u8]) -> RefBareItem<'a> {
        RefBareItem::ByteSeq(val)
    }
}

impl<'a, S, B> From<&'a Token> for GenericBareItem<S, B, &'a TokenRef> {
    fn from(val: &'a Token) -> Self {
        Self::Token(val)
    }
}

impl<'a, S, B> From<&'a TokenRef> for GenericBareItem<S, B, &'a TokenRef> {
    fn from(val: &'a TokenRef) -> Self {
        Self::Token(val)
    }
}

impl<'a> From<&'a String> for RefBareItem<'a> {
    fn from(val: &'a String) -> RefBareItem<'a> {
        RefBareItem::String(val)
    }
}

impl<'a> From<&'a StringRef> for RefBareItem<'a> {
    fn from(val: &'a StringRef) -> RefBareItem<'a> {
        RefBareItem::String(val)
    }
}

impl<S1, B1, T1, S2, B2, T2> PartialEq<GenericBareItem<S2, B2, T2>> for GenericBareItem<S1, B1, T1>
where
    for<'a> RefBareItem<'a>: From<&'a Self>,
    for<'a> RefBareItem<'a>: From<&'a GenericBareItem<S2, B2, T2>>,
{
    fn eq(&self, other: &GenericBareItem<S2, B2, T2>) -> bool {
        match (RefBareItem::from(self), RefBareItem::from(other)) {
            (RefBareItem::Integer(a), RefBareItem::Integer(b)) => a == b,
            (RefBareItem::Decimal(a), RefBareItem::Decimal(b)) => a == b,
            (RefBareItem::String(a), RefBareItem::String(b)) => a == b,
            (RefBareItem::ByteSeq(a), RefBareItem::ByteSeq(b)) => a == b,
            (RefBareItem::Boolean(a), RefBareItem::Boolean(b)) => a == b,
            (RefBareItem::Token(a), RefBareItem::Token(b)) => a == b,
            _ => false,
        }
    }
}
