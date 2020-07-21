/*!
`sfv` crate is an implementation of IETF draft [Structured Field Values for HTTP](https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html)
for parsing HTTP field values into structured values and serializing them.
It also exposes a set of types that might be useful for defining new structured fields.

# Data Structures

There are three types of structured fields:

- `Item` - can be an `Integer`, `Decimal`, `String`, `Token`, `Byte Sequence`, or `Boolean`. It can have associated `Parameters`.
- `List` - array of zero or more members, each of which can be an `Item` or an `InnerList`, both of which can be `Parameterized`.
- `Dictionary` - ordered map of name-value pairs, where the names are short textual strings and the values are `Items` or arrays of `Items` (represented with `InnerList`), both of which can be `Parameterized`. There can be zero or more members, and their names are unique in the scope of the `Dictionary` they occur within.

There's a number of primitive types used to construct structured field values:
- `BareItem` used as `Item`'s value or as a parameter value in `Parameters`.
- `Parameters` are an ordered map of key-value pairs that are associated with an `Item` or `InnerList`. The keys are unique within the scope the `Parameters` they occur within, and the values are `BareItem`.
- `InnerList` is an array of zero or more `Items`. Can have `Parameters`.
- `ListEntry` represents either `Item` or `InnerList` as a member of `List` or as member-value in `Dictionary`.

# Examples

### Parsing

```
use sfv::Parser;

// parsing structured field value of Item type
let item_header_input = "12.445;foo=bar";
let item = Parser::parse_item(item_header_input.as_bytes());
assert!(item.is_ok());
println!("{:#?}", item);

// parsing structured field value of List type
let list_header_input = "1;a=tok, (\"foo\" \"bar\");baz, ()";
let list = Parser::parse_list(list_header_input.as_bytes());
assert!(list.is_ok());
println!("{:#?}", list);

// parsing structured field value of Dictionary type
let dict_header_input = "a=?0, b, c; foo=bar, rating=1.5, fruits=(apple pear)";
let dict = Parser::parse_dictionary(dict_header_input.as_bytes());
assert!(dict.is_ok());
println!("{:#?}", dict);

```

### Structured Field Value Construction and Serialization
*/

mod parser;
mod serializer;
mod utils;

#[cfg(test)]
mod test_parser;
#[cfg(test)]
mod test_serializer;
use indexmap::IndexMap;

pub use rust_decimal::{
    prelude::{FromPrimitive, FromStr},
    Decimal,
};

pub use parser::{ParseMore, ParseValue, Parser};
pub use serializer::SerializeValue;

type SFVResult<T> = std::result::Result<T, &'static str>;

/// Represents Dictionary type structured field value.
pub type Dictionary = IndexMap<String, ListEntry>;

/// Represents List type structured field value.
pub type List = Vec<ListEntry>;

/// Parameters of Item or InnerList.
pub type Parameters = IndexMap<String, BareItem>;

/// Represents a member of List or Dictionary structured field value.
#[derive(Debug, PartialEq, Clone)]
pub enum ListEntry {
    /// Member of Item type.
    Item(Item),
    /// Member of InnerList (array of Items) type.
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

/// Array of Items with associated Parameters.
#[derive(Debug, PartialEq, Clone)]
pub struct InnerList {
    pub items: Vec<Item>,
    pub params: Parameters,
}

impl InnerList {
    fn new(items: Vec<Item>) -> InnerList {
        InnerList {
            items,
            params: Parameters::new(),
        }
    }

    pub fn with_params(items: Vec<Item>, params: Parameters) -> InnerList {
        InnerList { items, params }
    }
}

/// Represents List type structured field value.
/// Can be used as a member of List or Dictionary.
#[derive(Debug, PartialEq, Clone)]
pub struct Item {
    pub bare_item: BareItem,
    pub params: Parameters,
}

impl Item {
    fn new(bare_item: BareItem) -> Item {
        Item {
            bare_item,
            params: Parameters::new(),
        }
    }

    pub fn with_params(bare_item: BareItem, params: Parameters) -> Item {
        Item { bare_item, params }
    }
}

/// Numeric variant of BareItem.
#[derive(Debug, PartialEq, Clone)]
pub enum Num {
    /// Decimal number
    Decimal(Decimal),
    /// Integer number
    Integer(i64),
}

/// BareItem type is used to construct Items or Parameters values.
#[derive(Debug, PartialEq, Clone)]
pub enum BareItem {
    Number(Num),
    String(String),
    ByteSeq(Vec<u8>),
    Boolean(bool),
    Token(String),
}

impl From<i64> for BareItem {
    fn from(item: i64) -> Self {
        BareItem::Number(Num::Integer(item))
    }
}

impl From<Decimal> for BareItem {
    fn from(item: Decimal) -> Self {
        BareItem::Number(Num::Decimal(item))
    }
}
