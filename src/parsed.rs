use crate::{BareItem, Key};
use indexmap::IndexMap;

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
