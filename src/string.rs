use std::{borrow::Cow, string::String as StdString};

use crate::utils;

/// An owned structured field value [string].
///
/// Strings may only contain printable ASCII characters (i.e. the range
/// `0x20 ..= 0x7e`).
///
/// [string]: <https://httpwg.org/specs/rfc9651.html#string>
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct String(StdString);

/// A borrowed structured field value [string].
///
/// Strings may only contain printable ASCII characters (i.e. the range
/// `0x20 ..= 0x7e`).
///
/// This type is to [`String`] as [`str`] is to [`std::string::String`].
///
/// [string]: <https://httpwg.org/specs/rfc9651.html#string>
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, ref_cast::RefCastCustom)]
#[repr(transparent)]
pub struct StringRef(str);

impl_string_types!(
    owned: String,
    ref: StringRef,
    const_fn: string_ref,
    desc: "string",
    is_allowed_inner_char: utils::is_allowed_string_char,
);

impl StringRef {
    /// An empty `&StringRef`.
    pub const EMPTY: &Self = Self::constant("");
}

impl_eq!(StringRef: Cow<'_, StringRef>, StringRef);
impl_eq!(StringRef: Cow<'_, StringRef>, &StringRef);

impl Default for &StringRef {
    fn default() -> Self {
        StringRef::EMPTY
    }
}
