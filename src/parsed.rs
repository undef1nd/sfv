use std::convert::Infallible;

use indexmap::IndexMap;

use crate::{
    visitor::{
        DictionaryVisitor, EntryVisitor, InnerListVisitor, ItemVisitor, ListVisitor,
        ParameterVisitor,
    },
    BareItem, BareItemFromInput, Key, KeyRef,
};

/// An [item]-type structured field value.
///
/// Can be used as a member of `List` or `Dictionary`.
///
/// [item]: <https://httpwg.org/specs/rfc9651.html#item>
// sf-item   = bare-item parameters
// bare-item = sf-integer / sf-decimal / sf-string / sf-token
//             / sf-binary / sf-boolean
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Item {
    /// The item's value.
    pub bare_item: BareItem,
    /// The item's parameters, which can be empty.
    pub params: Parameters,
}

impl Item {
    /// Returns a new `Item` with empty `Parameters`.
    pub fn new(bare_item: impl Into<BareItem>) -> Self {
        Self {
            bare_item: bare_item.into(),
            params: Parameters::new(),
        }
    }

    /// Returns a new `Item` with the given `Parameters`.
    pub fn with_params(bare_item: impl Into<BareItem>, params: Parameters) -> Self {
        Self {
            bare_item: bare_item.into(),
            params,
        }
    }
}

/// A [dictionary]-type structured field value.
///
/// [dictionary]: <https://httpwg.org/specs/rfc9651.html#dictionary>
// sf-dictionary  = dict-member *( OWS "," OWS dict-member )
// dict-member    = member-name [ "=" member-value ]
// member-name    = key
// member-value   = sf-item / inner-list
pub type Dictionary = IndexMap<Key, ListEntry>;

/// A [list]-type structured field value.
///
/// [list]: <https://httpwg.org/specs/rfc9651.html#list>
// sf-list       = list-member *( OWS "," OWS list-member )
// list-member   = sf-item / inner-list
pub type List = Vec<ListEntry>;

/// [Parameters] of an [`Item`] or [`InnerList`].
///
/// [parameters]: <https://httpwg.org/specs/rfc9651.html#param>
// parameters    = *( ";" *SP parameter )
// parameter     = param-name [ "=" param-value ]
// param-name    = key
// key           = ( lcalpha / "*" )
//                 *( lcalpha / DIGIT / "_" / "-" / "." / "*" )
// lcalpha       = %x61-7A ; a-z
// param-value   = bare-item
pub type Parameters = IndexMap<Key, BareItem>;

/// A member of a [`List`] or [`Dictionary`].
#[derive(Debug, PartialEq, Clone)]
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

/// An [array] of [`Item`]s with associated [`Parameters`].
///
/// [array]: <https://httpwg.org/specs/rfc9651.html#inner-list>
// inner-list    = "(" *SP [ sf-item *( 1*SP sf-item ) *SP ] ")"
//                 parameters
#[derive(Debug, Default, PartialEq, Clone)]
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

impl<'a> ParameterVisitor<'a> for &mut Parameters {
    type Error = Infallible;

    fn parameter(
        &mut self,
        key: &'a KeyRef,
        value: BareItemFromInput<'a>,
    ) -> Result<(), Self::Error> {
        self.insert(key.to_owned(), value.into());
        Ok(())
    }
}

impl<'a> ItemVisitor<'a> for &mut Item {
    type Error = Infallible;

    fn bare_item<'pv>(
        self,
        bare_item: BareItemFromInput<'a>,
    ) -> Result<impl ParameterVisitor<'pv>, Self::Error> {
        self.bare_item = bare_item.into();
        Ok(&mut self.params)
    }
}

impl<'a> ItemVisitor<'a> for &mut InnerList {
    type Error = Infallible;
    fn bare_item<'pv>(
        self,
        bare_item: BareItemFromInput<'a>,
    ) -> Result<impl ParameterVisitor<'pv>, Self::Error> {
        self.items.push(Item::new(bare_item));
        match self.items.last_mut() {
            Some(item) => Ok(&mut item.params),
            None => unreachable!(),
        }
    }
}

impl InnerListVisitor<'_> for &mut InnerList {
    type Error = Infallible;

    fn item<'iv>(&mut self) -> Result<impl ItemVisitor<'iv>, Self::Error> {
        Ok(&mut **self)
    }

    fn finish<'pv>(self) -> Result<impl ParameterVisitor<'pv>, Self::Error> {
        Ok(&mut self.params)
    }
}

impl<'a> DictionaryVisitor<'a> for Dictionary {
    type Error = Infallible;

    fn entry<'dv, 'ev>(
        &'dv mut self,
        key: &'a KeyRef,
    ) -> Result<impl EntryVisitor<'ev>, Self::Error>
    where
        'dv: 'ev,
    {
        Ok(self.entry(key.to_owned()))
    }
}

type Entry<'a> = indexmap::map::Entry<'a, Key, ListEntry>;

impl<'a> ItemVisitor<'a> for Entry<'_> {
    type Error = Infallible;

    fn bare_item<'pv>(
        self,
        bare_item: BareItemFromInput<'a>,
    ) -> Result<impl ParameterVisitor<'pv>, Self::Error> {
        match self.insert_entry(Item::new(bare_item).into()).into_mut() {
            ListEntry::Item(item) => Ok(&mut item.params),
            ListEntry::InnerList(_) => unreachable!(),
        }
    }
}

impl EntryVisitor<'_> for Entry<'_> {
    fn inner_list<'ilv>(self) -> Result<impl InnerListVisitor<'ilv>, Self::Error> {
        match self.insert_entry(InnerList::default().into()).into_mut() {
            ListEntry::InnerList(inner_list) => Ok(inner_list),
            ListEntry::Item(_) => unreachable!(),
        }
    }
}

impl<'a> ItemVisitor<'a> for &mut List {
    type Error = Infallible;

    fn bare_item<'pv>(
        self,
        bare_item: BareItemFromInput<'a>,
    ) -> Result<impl ParameterVisitor<'pv>, Self::Error> {
        self.push(Item::new(bare_item).into());
        match self.last_mut() {
            Some(ListEntry::Item(item)) => Ok(&mut item.params),
            _ => unreachable!(),
        }
    }
}

impl EntryVisitor<'_> for &mut List {
    fn inner_list<'ilv>(self) -> Result<impl InnerListVisitor<'ilv>, Self::Error> {
        self.push(InnerList::default().into());
        match self.last_mut() {
            Some(ListEntry::InnerList(inner_list)) => Ok(inner_list),
            _ => unreachable!(),
        }
    }
}

impl ListVisitor<'_> for List {
    type Error = Infallible;

    fn entry<'ev>(&mut self) -> Result<impl EntryVisitor<'ev>, Self::Error> {
        Ok(self)
    }
}
