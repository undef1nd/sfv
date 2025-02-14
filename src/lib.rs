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
use sfv::{string_ref, Item, SerializeValue};

let str_item = Item::new(string_ref("foo").to_owned());
assert_eq!(str_item.serialize_value().unwrap(), r#""foo""#);
```


Creates `Item` field value with parameters:
```
use sfv::{Item, BareItem, SerializeValue, Parameters, Decimal, FromPrimitive};

let mut params = Parameters::new();
let decimal = Decimal::from_f64(13.45655).unwrap();
params.insert("key".into(), BareItem::Decimal(decimal));
let int_item = Item::with_params(99, params);
assert_eq!(int_item.serialize_value().unwrap(), "99;key=13.457");
```

Creates `List` field value with `Item` and parametrized `InnerList` as members:
```
use sfv::{string_ref, token_ref, Item, BareItem, InnerList, List, SerializeValue, Parameters};

let tok_item = BareItem::Token(token_ref("tok").to_owned());

// Creates Item.
let str_item = Item::new(string_ref("foo").to_owned());

// Creates InnerList members.
let mut int_item_params = Parameters::new();
int_item_params.insert("key".into(), BareItem::Boolean(false));
let int_item = Item::with_params(99, int_item_params);

// Creates InnerList.
let mut inner_list_params = Parameters::new();
inner_list_params.insert("bar".into(), BareItem::Boolean(true));
let inner_list = InnerList::with_params(vec![int_item, str_item], inner_list_params);


let list: List = vec![Item::new(tok_item).into(), inner_list.into()];
assert_eq!(
    list.serialize_value().unwrap(),
    r#"tok, (99;key=?0 "foo");bar"#
);
```

Creates `Dictionary` field value:
```
use sfv::{string_ref, Parser, Item, SerializeValue, Dictionary};

let member_value1 = Item::new(string_ref("apple").to_owned());
let member_value2 = Item::new(true);
let member_value3 = Item::new(false);

let mut dict = Dictionary::new();
dict.insert("key1".into(), member_value1.into());
dict.insert("key2".into(), member_value2.into());
dict.insert("key3".into(), member_value3.into());

assert_eq!(
    dict.serialize_value().unwrap(),
    r#"key1="apple", key2, key3=?0"#
);

```
*/

mod error;
mod integer;
mod parser;
mod ref_serializer;
mod serializer;
mod string;
mod token;
mod utils;

#[cfg(test)]
mod test_integer;
#[cfg(test)]
mod test_parser;
#[cfg(test)]
mod test_serializer;
#[cfg(test)]
mod test_string;
#[cfg(test)]
mod test_token;

use indexmap::IndexMap;
use std::string::String as StdString;

pub use rust_decimal::{
    prelude::{FromPrimitive, FromStr},
    Decimal,
};

pub use error::Error;
pub use integer::{integer, Integer, OutOfRangeError};
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
pub type Dictionary = IndexMap<StdString, ListEntry>;

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
pub type Parameters = IndexMap<StdString, BareItem>;

/// Represents a member of `List` or `Dictionary` structured field value.
#[derive(Debug, PartialEq, Clone)]
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
#[derive(Debug, PartialEq, Clone)]
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

/// `BareItem` type is used to construct `Items` or `Parameters` values.
#[derive(Debug, PartialEq, Clone)]
pub enum BareItem {
    /// Decimal number
    // sf-decimal  = ["-"] 1*12DIGIT "." 1*3DIGIT
    Decimal(Decimal),
    /// Integer number
    // sf-integer = ["-"] 1*15DIGIT
    Integer(Integer),
    // sf-string = DQUOTE *chr DQUOTE
    // chr       = unescaped / escaped
    // unescaped = %x20-21 / %x23-5B / %x5D-7E
    // escaped   = "\" ( DQUOTE / "\" )
    String(String),
    // ":" *(base64) ":"
    // base64    = ALPHA / DIGIT / "+" / "/" / "="
    ByteSeq(Vec<u8>),
    // sf-boolean = "?" boolean
    // boolean    = "0" / "1"
    Boolean(bool),
    // sf-token = ( ALPHA / "*" ) *( tchar / ":" / "/" )
    Token(Token),
}

impl BareItem {
    /// If `BareItem` is a decimal, returns `Decimal`, otherwise returns `None`.
    /// ```
    /// # use sfv::{BareItem, Decimal, FromPrimitive};
    /// let decimal_number = Decimal::from_f64(415.566).unwrap();
    /// let bare_item: BareItem = decimal_number.into();
    /// assert_eq!(bare_item.as_decimal().unwrap(), decimal_number);
    /// ```
    pub fn as_decimal(&self) -> Option<Decimal> {
        match *self {
            BareItem::Decimal(val) => Some(val),
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
            BareItem::Integer(val) => Some(val),
            _ => None,
        }
    }
    /// If `BareItem` is `String`, returns `&str`, otherwise returns `None`.
    /// ```
    /// # use sfv::{string_ref, BareItem};
    /// let bare_item = BareItem::String(string_ref("foo").to_owned());
    /// assert_eq!(bare_item.as_str().unwrap().as_str(), "foo");
    /// ```
    pub fn as_str(&self) -> Option<&String> {
        match *self {
            BareItem::String(ref val) => Some(val),
            _ => None,
        }
    }
    /// If `BareItem` is a `ByteSeq`, returns `&Vec<u8>`, otherwise returns `None`.
    /// ```
    /// # use sfv::BareItem;
    /// let bare_item = BareItem::ByteSeq("foo".to_owned().into_bytes());
    /// assert_eq!(bare_item.as_byte_seq().unwrap().as_slice(), "foo".as_bytes());
    /// ```
    pub fn as_byte_seq(&self) -> Option<&Vec<u8>> {
        match *self {
            BareItem::ByteSeq(ref val) => Some(val),
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
            BareItem::Boolean(val) => Some(val),
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
    pub fn as_token(&self) -> Option<&Token> {
        match *self {
            BareItem::Token(ref val) => Some(val),
            _ => None,
        }
    }
}

impl From<Integer> for BareItem {
    fn from(val: Integer) -> BareItem {
        BareItem::Integer(val)
    }
}

impl From<bool> for BareItem {
    fn from(val: bool) -> BareItem {
        BareItem::Boolean(val)
    }
}

impl From<Decimal> for BareItem {
    /// Converts `Decimal` into `BareItem::Decimal`.
    /// ```
    /// # use sfv::{BareItem, Decimal, FromPrimitive};
    /// let decimal_number = Decimal::from_f64(48.01).unwrap();
    /// let bare_item: BareItem = decimal_number.into();
    /// assert_eq!(bare_item.as_decimal().unwrap(), decimal_number);
    /// ```
    fn from(item: Decimal) -> Self {
        BareItem::Decimal(item)
    }
}

impl From<Vec<u8>> for BareItem {
    fn from(val: Vec<u8>) -> BareItem {
        BareItem::ByteSeq(val)
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

#[derive(Debug, PartialEq)]
pub(crate) enum Num {
    Decimal(Decimal),
    Integer(Integer),
}

/// Similar to `BareItem`, but used to serialize values via `RefItemSerializer`, `RefListSerializer`, `RefDictSerializer`.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RefBareItem<'a> {
    Integer(Integer),
    Decimal(Decimal),
    String(&'a StringRef),
    ByteSeq(&'a [u8]),
    Boolean(bool),
    Token(&'a TokenRef),
}

impl<'a> From<&'a BareItem> for RefBareItem<'a> {
    fn from(val: &'a BareItem) -> RefBareItem<'a> {
        match val {
            BareItem::Integer(val) => RefBareItem::Integer(*val),
            BareItem::Decimal(val) => RefBareItem::Decimal(*val),
            BareItem::String(val) => RefBareItem::String(val),
            BareItem::ByteSeq(val) => RefBareItem::ByteSeq(val),
            BareItem::Boolean(val) => RefBareItem::Boolean(*val),
            BareItem::Token(val) => RefBareItem::Token(val),
        }
    }
}

impl<'a> From<Integer> for RefBareItem<'a> {
    fn from(val: Integer) -> RefBareItem<'a> {
        RefBareItem::Integer(val)
    }
}

impl<'a> From<bool> for RefBareItem<'a> {
    fn from(val: bool) -> RefBareItem<'a> {
        RefBareItem::Boolean(val)
    }
}

impl<'a> From<Decimal> for RefBareItem<'a> {
    fn from(val: Decimal) -> RefBareItem<'a> {
        RefBareItem::Decimal(val)
    }
}

impl<'a> From<&'a [u8]> for RefBareItem<'a> {
    fn from(val: &'a [u8]) -> RefBareItem<'a> {
        RefBareItem::ByteSeq(val)
    }
}

impl<'a> From<&'a Token> for RefBareItem<'a> {
    fn from(val: &'a Token) -> RefBareItem<'a> {
        RefBareItem::Token(val)
    }
}

impl<'a> From<&'a TokenRef> for RefBareItem<'a> {
    fn from(val: &'a TokenRef) -> RefBareItem<'a> {
        RefBareItem::Token(val)
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
