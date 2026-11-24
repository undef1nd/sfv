use indexmap::IndexMap;

use crate::{
    BareItem, BareItemFromInput, Error, Key, KeyRef, Parser,
    private::Sealed,
    visitor::{
        BareItemVisitor, DictionaryVisitor, EntryVisitor, InnerListVisitor, ItemVisitor,
        ListVisitor, ParameterVisitor,
    },
};

/// An [item]-type structured field value.
///
/// Can be used as a member of `List` or `Dictionary`.
///
/// [item]: <https://httpwg.org/specs/rfc9651.html#item>
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Item {
    /// The item's value.
    pub bare_item: BareItem,
    /// The item's parameters, which can be empty.
    pub params: Parameters,
}

impl Item {
    /// Returns a new `Item` with empty `Parameters`.
    #[must_use]
    pub fn new(bare_item: impl Into<BareItem>) -> Self {
        Self {
            bare_item: bare_item.into(),
            params: Parameters::new(),
        }
    }

    /// Returns a new `Item` with the given `Parameters`.
    #[must_use]
    pub fn with_params(bare_item: impl Into<BareItem>, params: Parameters) -> Self {
        Self {
            bare_item: bare_item.into(),
            params,
        }
    }
}

impl<T> From<T> for Item
where
    T: Into<BareItem>,
{
    /// Converts a value into an [`Item`] with no parameters.
    fn from(bare_item: T) -> Self {
        Self::new(bare_item)
    }
}

/// A [dictionary]-type structured field value.
///
/// [dictionary]: <https://httpwg.org/specs/rfc9651.html#dictionary>
pub type Dictionary = IndexMap<Key, ListEntry>;

/// A [list]-type structured field value.
///
/// [list]: <https://httpwg.org/specs/rfc9651.html#list>
pub type List = Vec<ListEntry>;

/// [Parameters] of an [`Item`] or [`InnerList`].
///
/// [parameters]: <https://httpwg.org/specs/rfc9651.html#param>
pub type Parameters = IndexMap<Key, BareItem>;

/// A member of a [`List`] or [`Dictionary`].
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum ListEntry {
    /// An item.
    Item(Item),
    /// An inner list.
    InnerList(InnerList),
}

impl From<Item> for ListEntry {
    fn from(item: Item) -> Self {
        ListEntry::Item(item)
    }
}

impl From<InnerList> for ListEntry {
    fn from(inner_list: InnerList) -> Self {
        ListEntry::InnerList(inner_list)
    }
}

impl<T> From<T> for ListEntry
where
    T: Into<BareItem>,
{
    /// Converts a value into a [`ListEntry::Item`] with no parameters.
    fn from(bare_item: T) -> Self {
        ListEntry::Item(Item::new(bare_item))
    }
}

/// An [array] of [`Item`]s with associated [`Parameters`].
///
/// [array]: <https://httpwg.org/specs/rfc9651.html#inner-list>
#[derive(Debug, Default, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct InnerList {
    /// The inner list's items, which can be empty.
    pub items: Vec<Item>,
    /// The inner list's parameters, which can be empty.
    pub params: Parameters,
}

impl InnerList {
    /// Returns a new `InnerList` with empty `Parameters`.
    #[must_use]
    pub fn new(items: Vec<Item>) -> Self {
        Self {
            items,
            params: Parameters::new(),
        }
    }

    /// Returns a new `InnerList` with the given `Parameters`.
    #[must_use]
    pub fn with_params(items: Vec<Item>, params: Parameters) -> Self {
        Self { items, params }
    }
}

// Used to hide the visitor impls for public types.
struct Wrapper<T>(T);

impl<'de> BareItemVisitor<'de> for Wrapper<&mut BareItem> {
    type Out = ();

    fn bare_item(self, value: BareItemFromInput<'de>) -> Self::Out {
        *self.0 = value.into_bare_item();
    }
}

impl<'de> ParameterVisitor<'de> for Wrapper<&mut Parameters> {
    type Out = ();

    fn parameter(&mut self, key: &'de KeyRef) -> impl BareItemVisitor<'de> {
        // TODO: Avoid allocating an owned `Key` when an equivalent key is
        // already present if `IndexMap` adds an `entry_ref`-style API.
        Wrapper(
            self.0
                .entry(key.to_owned())
                .or_insert(BareItem::Boolean(false)),
        )
    }

    fn finish(self) -> Self::Out {}
}

impl<'de, 'a> BareItemVisitor<'de> for Wrapper<&'a mut Item> {
    type Out = Wrapper<&'a mut Parameters>;

    fn bare_item(self, bare_item: BareItemFromInput<'de>) -> Self::Out {
        Wrapper(&mut self.0.bare_item).bare_item(bare_item);
        Wrapper(&mut self.0.params)
    }
}

impl<'de> InnerListVisitor<'de> for Wrapper<&mut InnerList> {
    fn item(&mut self) -> impl ItemVisitor<'de> {
        Wrapper(self.0.items.push_mut(Item::new(false)))
    }

    fn finish(self) -> impl ParameterVisitor<'de> {
        Wrapper(&mut self.0.params)
    }
}

impl<'de> EntryVisitor<'de> for Wrapper<&mut ListEntry> {
    fn item(self) -> impl ItemVisitor<'de> {
        loop {
            match *self.0 {
                ListEntry::Item(ref mut item) => {
                    item.params.clear();
                    return Wrapper(item);
                }
                ListEntry::InnerList(_) => {
                    *self.0 = ListEntry::Item(Item::new(false));
                }
            }
        }
    }

    fn inner_list(self) -> impl InnerListVisitor<'de> {
        loop {
            match *self.0 {
                ListEntry::InnerList(ref mut inner_list) => {
                    inner_list.items.clear();
                    inner_list.params.clear();
                    return Wrapper(inner_list);
                }
                ListEntry::Item(_) => {
                    *self.0 = ListEntry::InnerList(InnerList::default());
                }
            }
        }
    }
}

impl<'de> DictionaryVisitor<'de> for Wrapper<&mut Dictionary> {
    type Out = ();

    fn entry(&mut self, key: &'de KeyRef) -> impl EntryVisitor<'de> {
        // TODO: Avoid allocating an owned `Key` when an equivalent key is
        // already present if `IndexMap` adds an `entry_ref`-style API.
        Wrapper(
            self.0
                .entry(key.to_owned())
                .or_insert_with(|| Item::new(false).into()),
        )
    }

    fn finish(self) -> Self::Out {}
}

impl<'de> ListVisitor<'de> for Wrapper<&mut List> {
    type Out = ();

    fn entry(&mut self) -> impl EntryVisitor<'de> {
        Wrapper(self.0.push_mut(Item::new(false).into()))
    }

    fn finish(self) -> Self::Out {}
}

/// A structured-field type, supporting parsing and serialization.
pub trait FieldType: Sealed {
    /// The result of serializing the value into a string.
    ///
    /// [`Item`] serialization is infallible; [`List`] and [`Dictionary`]
    /// serialization is not.
    type SerializeResult: Into<Option<String>>;

    /// Serializes a structured field value into a string.
    ///
    /// Note: The serialization conforms to [RFC 9651], meaning that
    /// [`Dates`][crate::Date] and [`Display Strings`][crate::RefBareItem::DisplayString],
    /// which cause parsing errors under [RFC 8941], will be serialized
    /// unconditionally. The consumer of this API is responsible for determining
    /// whether it is valid to serialize these bare items for any specific field.
    ///
    /// [RFC 8941]: <https://httpwg.org/specs/rfc8941.html>
    /// [RFC 9651]: <https://httpwg.org/specs/rfc9651.html>
    ///
    /// Use [`crate::ItemSerializer`], [`crate::ListSerializer`], or
    /// [`crate::DictSerializer`] to serialize components incrementally without
    /// having to create an [`Item`], [`List`], or [`Dictionary`].
    #[must_use]
    fn serialize(&self) -> Self::SerializeResult;

    /// Parses a structured-field value from the given parser.
    ///
    /// # Errors
    /// When the parsing process is unsuccessful.
    fn parse(parser: Parser<'_>) -> Result<Self, Error>
    where
        Self: Sized;
}

impl Sealed for Item {}

impl FieldType for Item {
    type SerializeResult = String;

    fn serialize(&self) -> String {
        crate::ItemSerializer::new()
            .bare_item(&self.bare_item)
            .parameters(&self.params)
            .finish()
    }

    fn parse(parser: Parser<'_>) -> Result<Self, Error> {
        let mut item = Self::new(false);
        parser.parse_item_with_visitor(Wrapper(&mut item))?;
        Ok(item)
    }
}

impl Sealed for List {}

impl FieldType for List {
    type SerializeResult = Option<String>;

    fn serialize(&self) -> Option<String> {
        let mut ser = crate::ListSerializer::new();
        ser.members(self);
        ser.finish()
    }

    fn parse(parser: Parser<'_>) -> Result<Self, Error> {
        let mut list = Self::new();
        parser.parse_list_with_visitor(Wrapper(&mut list))?;
        Ok(list)
    }
}

impl Sealed for Dictionary {}

impl FieldType for Dictionary {
    type SerializeResult = Option<String>;

    fn serialize(&self) -> Option<String> {
        let mut ser = crate::DictSerializer::new();
        ser.members(self);
        ser.finish()
    }

    fn parse(parser: Parser<'_>) -> Result<Self, Error> {
        let mut dict = Self::new();
        parser.parse_dictionary_with_visitor(Wrapper(&mut dict))?;
        Ok(dict)
    }
}
