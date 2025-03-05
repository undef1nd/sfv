/*!
`sfv` crate is an implementation of *Structured Field Values for HTTP* as specified in [RFC 8941](https://httpwg.org/specs/rfc8941.html) for parsing and serializing HTTP field values.
It also exposes a set of types that might be useful for defining new structured fields.

# Data Structures

There are three types of structured fields:

- `Item` - can be an `Integer`, `Decimal`, `String`, `Token`, `Byte Sequence`, or `Boolean`. It can have associated `Parameters`.
- `List` - array of zero or more members, each of which can be an `Item` or an `InnerList`, both of which can be `Parameterized`.
- `Dictionary` - ordered map of name-value pairs, where the names are short textual strings and the values are `Items` or arrays of `Items` (represented with `InnerList`), both of which can be `Parameterized`. There can be zero or more members, and their names are unique in the scope of the `Dictionary` they occur within.

There's also a few primitive types used to construct structured field values:
- `BareItem` used as `Item`'s value or as a parameter value in `Parameters`.
- `Parameters` are an ordered map of key-value pairs that are associated with an `Item` or `InnerList`. The keys are unique within the scope the `Parameters` they occur within, and the values are `BareItem`.
- `InnerList` is an array of zero or more `Items`. Can have `Parameters`.
- `ListEntry` represents either `Item` or `InnerList` as a member of `List` or as member-value in `Dictionary`.

# Examples

### Parsing

```
use sfv::Parser;

// Parsing structured field value of Item type.
let item_header_input = "12.445;foo=bar";
let item = Parser::from_str(item_header_input).parse_item();
assert!(item.is_ok());
println!("{:#?}", item);

// Parsing structured field value of List type.
let list_header_input = r#"1;a=tok, ("foo" "bar");baz, ()"#;
let list = Parser::from_str(list_header_input).parse_list();
assert!(list.is_ok());
println!("{:#?}", list);

// Parsing structured field value of Dictionary type.
let dict_header_input = "a=?0, b, c; foo=bar, rating=1.5, fruits=(apple pear)";
let dict = Parser::from_str(dict_header_input).parse_dictionary();
assert!(dict.is_ok());
println!("{:#?}", dict);
```

### Getting Parsed Value Members
```
use sfv::*;

let dict_header = "u=2, n=(* foo 2)";
let dict = Parser::from_str(dict_header).parse_dictionary().unwrap();

// Case 1 - handling value if it's an Item of Integer type
let u_val = match dict.get("u") {
    Some(ListEntry::Item(item)) => item.bare_item.as_int(),
    _ => None,
};

if let Some(u_val) = u_val {
    println!("{}", u_val);
}

// Case 2 - matching on all possible types
match dict.get("u") {
    Some(ListEntry::Item(item)) => match &item.bare_item {
        BareItem::Token(val) => {
            // do something if it's a Token
            println!("{}", val);
        }
        BareItem::Integer(val) => {
            // do something if it's an Integer
            println!("{}", val);
        }
        BareItem::Boolean(val) => {
            // do something if it's a Boolean
            println!("{}", val);
        }
        BareItem::Decimal(val) => {
            // do something if it's a Decimal
            println!("{}", val);
        }
        BareItem::String(val) => {
            // do something if it's a String
            println!("{}", val);
        }
        BareItem::ByteSeq(val) => {
            // do something if it's a ByteSeq
            println!("{:?}", val);
        }
    },
    Some(ListEntry::InnerList(inner_list)) => {
        // do something if it's an InnerList
        println!("{:?}", inner_list.items);
    }
    None => panic!("key not found"),
}
```

### Structured Field Value Construction and Serialization
Creates `Item` with empty parameters:
```
use sfv::{StringRef, Item, SerializeValue};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let str_item = Item::new(StringRef::from_str("foo")?);
assert_eq!(str_item.serialize_value()?, r#""foo""#);
# Ok(())
# }
```


Creates `Item` field value with parameters:
```
use std::convert::TryFrom;
use sfv::{KeyRef, Item, BareItem, SerializeValue, Parameters, Decimal};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut params = Parameters::new();
let decimal = Decimal::try_from(13.45655)?;
params.insert(KeyRef::from_str("key")?.to_owned(), BareItem::Decimal(decimal));
let int_item = Item::with_params(99, params);
assert_eq!(int_item.serialize_value()?, "99;key=13.457");
# Ok(())
# }
```

Creates `List` field value with `Item` and parametrized `InnerList` as members:
```
use sfv::{KeyRef, StringRef, TokenRef, Item, BareItem, InnerList, List, SerializeValue, Parameters};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let tok_item = BareItem::Token(TokenRef::from_str("tok")?.to_owned());

// Creates Item.
let str_item = Item::new(StringRef::from_str("foo")?);

// Creates InnerList members.
let mut int_item_params = Parameters::new();
int_item_params.insert(KeyRef::from_str("key")?.to_owned(), BareItem::Boolean(false));
let int_item = Item::with_params(99, int_item_params);

// Creates InnerList.
let mut inner_list_params = Parameters::new();
inner_list_params.insert(KeyRef::from_str("bar")?.to_owned(), BareItem::Boolean(true));
let inner_list = InnerList::with_params(vec![int_item, str_item], inner_list_params);

let list: List = vec![Item::new(tok_item).into(), inner_list.into()];
assert_eq!(
    list.serialize_value()?,
    r#"tok, (99;key=?0 "foo");bar"#
);
# Ok(())
# }
```

Creates `Dictionary` field value:
```
use sfv::{KeyRef, StringRef, Parser, Item, SerializeValue, Dictionary};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let member_value1 = Item::new(StringRef::from_str("apple")?.to_owned());
let member_value2 = Item::new(true);
let member_value3 = Item::new(false);

let mut dict = Dictionary::new();
dict.insert(KeyRef::from_str("key1")?.to_owned(), member_value1.into());
dict.insert(KeyRef::from_str("key2")?.to_owned(), member_value2.into());
dict.insert(KeyRef::from_str("key3")?.to_owned(), member_value3.into());

assert_eq!(
    dict.serialize_value()?,
    r#"key1="apple", key2, key3=?0"#
);
# Ok(())
# }
```
*/

mod decimal;
mod error;
mod integer;
mod key;
mod parser;
mod ref_serializer;
mod serializer;
mod string;
mod token;
mod utils;

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

use indexmap::IndexMap;
use std::borrow::{Borrow, Cow};
use std::convert::TryFrom;

pub use decimal::{Decimal, DecimalError};
pub use error::Error;
pub use integer::{integer, Integer, OutOfRangeError};
pub use key::{key_ref, Key, KeyError, KeyRef};
pub use parser::{ParseMore, Parser};
pub use ref_serializer::{
    RefDictSerializer, RefInnerListSerializer, RefItemSerializer, RefListSerializer,
    RefParameterSerializer,
};
pub use serializer::SerializeValue;
pub use string::{string_ref, String, StringError, StringRef};
pub use token::{token_ref, Token, TokenError, TokenRef};

type SFVResult<T> = std::result::Result<T, Error>;

/// Represents `Item` type structured field value.
/// Can be used as a member of `List` or `Dictionary`.
// sf-item   = bare-item parameters
// bare-item = sf-integer / sf-decimal / sf-string / sf-token
//             / sf-binary / sf-boolean
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Item {
    /// Value of `Item`.
    pub bare_item: BareItem,
    /// `Item`'s associated parameters. Can be empty.
    pub params: Parameters,
}

impl Item {
    /// Returns new `Item` with empty `Parameters`.
    pub fn new(bare_item: impl Into<BareItem>) -> Item {
        Item {
            bare_item: bare_item.into(),
            params: Parameters::new(),
        }
    }
    /// Returns new `Item` with specified `Parameters`.
    pub fn with_params(bare_item: impl Into<BareItem>, params: Parameters) -> Item {
        Item {
            bare_item: bare_item.into(),
            params,
        }
    }
}

/// Represents `Dictionary` type structured field value.
// sf-dictionary  = dict-member *( OWS "," OWS dict-member )
// dict-member    = member-name [ "=" member-value ]
// member-name    = key
// member-value   = sf-item / inner-list
pub type Dictionary = IndexMap<Key, ListEntry>;

/// Represents `List` type structured field value.
// sf-list       = list-member *( OWS "," OWS list-member )
// list-member   = sf-item / inner-list
pub type List = Vec<ListEntry>;

/// Parameters of `Item` or `InnerList`.
// parameters    = *( ";" *SP parameter )
// parameter     = param-name [ "=" param-value ]
// param-name    = key
// key           = ( lcalpha / "*" )
//                 *( lcalpha / DIGIT / "_" / "-" / "." / "*" )
// lcalpha       = %x61-7A ; a-z
// param-value   = bare-item
pub type Parameters = IndexMap<Key, BareItem>;

/// Represents a member of `List` or `Dictionary` structured field value.
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum ListEntry {
    /// Member of `Item` type.
    Item(Item),
    /// Member of `InnerList` (array of `Items`) type.
    InnerList(InnerList),
}

impl From<Item> for ListEntry {
    fn from(item: Item) -> Self {
        ListEntry::Item(item)
    }
}

impl From<InnerList> for ListEntry {
    fn from(item: InnerList) -> Self {
        ListEntry::InnerList(item)
    }
}

/// Array of `Items` with associated `Parameters`.
// inner-list    = "(" *SP [ sf-item *( 1*SP sf-item ) *SP ] ")"
//                 parameters
#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct InnerList {
    /// `Items` that `InnerList` contains. Can be empty.
    pub items: Vec<Item>,
    /// `InnerList`'s associated parameters. Can be empty.
    pub params: Parameters,
}

impl InnerList {
    /// Returns new `InnerList` with empty `Parameters`.
    pub fn new(items: Vec<Item>) -> InnerList {
        InnerList {
            items,
            params: Parameters::new(),
        }
    }

    /// Returns new `InnerList` with specified `Parameters`.
    pub fn with_params(items: Vec<Item>, params: Parameters) -> InnerList {
        InnerList { items, params }
    }
}

/// An abstraction over multiple kinds of ownership of a bare item.
///
/// In general most users will be interested in:
/// - [`BareItem`], for completely owned data
/// - [`RefBareItem`], for completely borrowed data
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
    /// If `BareItem` is a decimal, returns `Decimal`, otherwise returns `None`.
    /// ```
    /// # use std::convert::TryFrom;
    /// # use sfv::{BareItem, Decimal};
    /// let decimal_number = Decimal::try_from(415.566).unwrap();
    /// let bare_item: BareItem = decimal_number.into();
    /// assert_eq!(bare_item.as_decimal().unwrap(), decimal_number);
    /// ```
    pub fn as_decimal(&self) -> Option<Decimal> {
        match *self {
            Self::Decimal(val) => Some(val),
            _ => None,
        }
    }
    /// If `BareItem` is an integer, returns `Integer`, otherwise returns `None`.
    /// ```
    /// # use sfv::{integer, BareItem};
    /// let bare_item: BareItem = 100.into();
    /// assert_eq!(bare_item.as_int().unwrap(), integer(100));
    /// ```
    pub fn as_int(&self) -> Option<Integer> {
        match *self {
            Self::Integer(val) => Some(val),
            _ => None,
        }
    }
    /// If `BareItem` is `String`, returns `&str`, otherwise returns `None`.
    /// ```
    /// # use sfv::{string_ref, BareItem};
    /// let bare_item = BareItem::String(string_ref("foo").to_owned());
    /// assert_eq!(bare_item.as_str().unwrap().as_str(), "foo");
    /// ```
    pub fn as_str(&self) -> Option<&S> {
        match *self {
            Self::String(ref val) => Some(val),
            _ => None,
        }
    }
    /// If `BareItem` is a `ByteSeq`, returns `&Vec<u8>`, otherwise returns `None`.
    /// ```
    /// # use sfv::BareItem;
    /// let bare_item = BareItem::ByteSeq(b"foo".to_vec());
    /// assert_eq!(bare_item.as_byte_seq().unwrap().as_slice(), "foo".as_bytes());
    /// ```
    pub fn as_byte_seq(&self) -> Option<&B> {
        match *self {
            Self::ByteSeq(ref val) => Some(val),
            _ => None,
        }
    }
    /// If `BareItem` is a `Boolean`, returns `bool`, otherwise returns `None`.
    /// ```
    /// # use sfv::{BareItem};
    /// let bare_item = BareItem::Boolean(true);
    /// assert_eq!(bare_item.as_bool().unwrap(), true);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Boolean(val) => Some(val),
            _ => None,
        }
    }
    /// If `BareItem` is a `Token`, returns `&Token`, otherwise returns `None`.
    /// ```
    /// use sfv::{token_ref, BareItem};
    ///
    /// let bare_item = BareItem::Token(token_ref("*bar").to_owned());
    /// assert_eq!(bare_item.as_token().unwrap().as_str(), "*bar");
    /// ```
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
    /// Converts `Decimal` into `BareItem::Decimal`.
    /// ```
    /// # use std::convert::TryFrom;
    /// # use sfv::{BareItem, Decimal};
    /// let decimal_number = Decimal::try_from(48.01).unwrap();
    /// let bare_item: BareItem = decimal_number.into();
    /// assert_eq!(bare_item.as_decimal().unwrap(), decimal_number);
    /// ```
    fn from(item: Decimal) -> Self {
        Self::Decimal(item)
    }
}

impl<S, B, T> TryFrom<f32> for GenericBareItem<S, B, T> {
    type Error = DecimalError;

    fn try_from(val: f32) -> Result<Self, DecimalError> {
        Decimal::try_from(val).map(Self::Decimal)
    }
}

impl<S, B, T> TryFrom<f64> for GenericBareItem<S, B, T> {
    type Error = DecimalError;

    fn try_from(val: f64) -> Result<Self, DecimalError> {
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

/// `BareItem` type is used to construct `Items` or `Parameters` values.
pub type BareItem = GenericBareItem<String, Vec<u8>, Token>;

/// Similar to `BareItem`, but used to serialize values via `RefItemSerializer`, `RefListSerializer`, `RefDictSerializer`.
pub type RefBareItem<'a> = GenericBareItem<&'a StringRef, &'a [u8], &'a TokenRef>;

/// Similar to `BareItem`, but borrows data from input when possible.
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
