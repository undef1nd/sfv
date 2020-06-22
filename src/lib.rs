mod utils;
use indexmap::IndexMap;
pub use rust_decimal::Decimal;

pub mod parser;
pub mod serializer;
pub(crate) mod test_parser;
pub(crate) mod test_serializer;

// Alias for Result with &'static str type Error
// std Result is used in tests
type Res<T> = Result<T, &'static str>;

pub type Dictionary = IndexMap<String, ListEntry>;
pub type Parameters = IndexMap<String, BareItem>;
pub type List = Vec<ListEntry>;

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct InnerList(pub Vec<Item>, pub Parameters);

#[derive(Debug, PartialEq)]
pub struct Item(pub BareItem, pub Parameters);

#[derive(Debug, PartialEq)]
pub enum Num {
    Decimal(Decimal),
    Integer(i64),
}

#[derive(Debug, PartialEq)]
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
