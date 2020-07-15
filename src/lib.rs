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

// Alias for Result with &'static str as Err
// std::result::Result is used in tests
type Result<T> = std::result::Result<T, &'static str>;

pub type Dictionary = IndexMap<String, ListEntry>;
pub type Parameters = IndexMap<String, BareItem>;
pub type List = Vec<ListEntry>;

#[derive(Debug, PartialEq, Clone)]
pub enum ListEntry {
    Item(Item),
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

#[derive(Debug, PartialEq, Clone)]
pub struct InnerList(pub Vec<Item>, pub Parameters);

#[derive(Debug, PartialEq, Clone)]
pub struct Item(pub BareItem, pub Parameters);

#[derive(Debug, PartialEq, Clone)]
pub enum Num {
    Decimal(Decimal),
    Integer(i64),
}

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
