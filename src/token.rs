use crate::utils;

/// An owned structured field value [token].
///
/// Tokens must match the following regular expression:
///
/// ```re
/// ^[A-Za-z*][A-Za-z*0-9!#$%&'+\-.^_`|~:/]*$
/// ```
///
/// [token]: <https://httpwg.org/specs/rfc9651.html#token>
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token(String);

/// A borrowed structured field value [token].
///
/// Tokens must match the following regular expression:
///
/// ```re
/// ^[A-Za-z*][A-Za-z*0-9!#$%&'+\-.^_`|~:/]*$
/// ```
///
/// This type is to [`Token`] as [`str`] is to [`String`].
///
/// [token]: <https://httpwg.org/specs/rfc9651.html#token>
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, ref_cast::RefCastCustom)]
#[repr(transparent)]
pub struct TokenRef(str);

impl_string_types!(
    owned: Token,
    ref: TokenRef,
    const_fn: token_ref,
    desc: "token",
    is_allowed_start_char: utils::is_allowed_start_token_char,
    is_allowed_inner_char: utils::is_allowed_inner_token_char,
);
