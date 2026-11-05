use crate::utils;

/// An owned structured field value [key].
///
/// Keys must match the following regular expression:
///
/// ```re
/// ^[a-z*][a-z*0-9_\-.]*$
/// ```
///
/// [key]: <https://httpwg.org/specs/rfc9651.html#key>
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(String);

/// A borrowed structured field value [key].
///
/// Keys must match the following regular expression:
///
/// ```re
/// ^[a-z*][a-z*0-9_\-.]*$
/// ```
///
/// This type is to [`Key`] as [`str`] is to [`String`].
///
/// [key]: <https://httpwg.org/specs/rfc9651.html#key>
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, ref_cast::RefCastCustom)]
#[repr(transparent)]
pub struct KeyRef(str);

impl_string_types!(
    owned: Key,
    ref: KeyRef,
    const_fn: key_ref,
    desc: "key",
    is_allowed_start_char: utils::is_allowed_start_key_char,
    is_allowed_inner_char: utils::is_allowed_inner_key_char,
);

impl AsRef<KeyRef> for Key {
    fn as_ref(&self) -> &KeyRef {
        self
    }
}

impl AsRef<KeyRef> for KeyRef {
    fn as_ref(&self) -> &KeyRef {
        self
    }
}
