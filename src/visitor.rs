/*!
Contains traits for parsing structured-field values incrementally.

These can be used to borrow data from the input without copies in some cases.
*/

use crate::{BareItemFromInput, KeyRef};
use std::convert::Infallible;
use std::error::Error;

/// A visitor whose methods are called during parameter parsing.
///
/// The lifetime `'a` is the lifetime of the input.
pub trait ParameterVisitor<'a> {
    type Error: Error;

    /// Called after a parameter has been parsed.
    ///
    /// Parsing will be terminated early if an error is returned.
    fn parameter(
        &mut self,
        key: &'a KeyRef,
        value: BareItemFromInput<'a>,
    ) -> Result<(), Self::Error>;

    /// Called after all parameters have been parsed.
    ///
    /// Parsing will be terminated early if an error is returned.
    fn finish(self) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        Ok(())
    }
}

/// A visitor whose methods are called during item parsing.
///
/// The lifetime `'a` is the lifetime of the input.
///
/// Use this trait with
/// [`Parser::parse_item_with_visitor`][crate::Parser::parse_item_with_visitor].
pub trait ItemVisitor<'a> {
    type Error: Error;

    /// Called after a bare item has been parsed.
    ///
    /// The returned visitor is used to handle the bare item's parameters.
    /// Return [`Ignored`] to silently discard all parameters.
    ///
    /// Parsing will be terminated early if an error is returned.
    fn bare_item(
        self,
        bare_item: BareItemFromInput<'a>,
    ) -> Result<impl ParameterVisitor<'a>, Self::Error>;
}

/// A visitor whose methods are called during inner-list parsing.
///
/// The lifetime `'a` is the lifetime of the input.
pub trait InnerListVisitor<'a> {
    type Error: Error;

    /// Called before an item has been parsed.
    ///
    /// The returned visitor is used to handle the bare item.
    ///
    /// Parsing will be terminated early if an error is returned.
    fn item(&mut self) -> Result<impl ItemVisitor<'a>, Self::Error>;

    /// Called after all inner-list items have been parsed.
    ///
    /// The returned visitor is used to handle the inner list's parameters.
    /// Return [`Ignored`] to silently discard all parameters.
    ///
    /// Parsing will be terminated early if an error is returned.
    fn finish(self) -> Result<impl ParameterVisitor<'a>, Self::Error>;
}

/// A visitor whose methods are called during entry parsing.
///
/// The lifetime `'a` is the lifetime of the input.
pub trait EntryVisitor<'a>: ItemVisitor<'a> {
    /// Called before an inner list has been parsed.
    ///
    /// The returned visitor is used to handle the inner list.
    ///
    /// Parsing will be terminated early if an error is returned.
    fn inner_list(self) -> Result<impl InnerListVisitor<'a>, Self::Error>;
}

/// A visitor whose methods are called during dictionary parsing.
///
/// The lifetime `'a` is the lifetime of the input.
///
/// Use this trait with
/// [`Parser::parse_dictionary_with_visitor`][crate::Parser::parse_dictionary_with_visitor].
pub trait DictionaryVisitor<'a> {
    type Error: Error;

    /// Called after a dictionary key has been parsed.
    ///
    /// The returned visitor is used to handle the associated value.
    ///
    /// Parsing will be terminated early if an error is returned.
    fn entry(&mut self, key: &'a KeyRef) -> Result<impl EntryVisitor<'a>, Self::Error>;
}

/// A visitor whose methods are called during list parsing.
///
/// The lifetime `'a` is the lifetime of the input.
///
/// Use this trait with
/// [`Parser::parse_list_with_visitor`][crate::Parser::parse_list_with_visitor].
pub trait ListVisitor<'a> {
    type Error: Error;

    /// Called before a list entry has been parsed.
    ///
    /// The returned visitor is used to handle the entry.
    ///
    /// Parsing will be terminated early if an error is returned.
    fn entry(&mut self) -> Result<impl EntryVisitor<'a>, Self::Error>;
}

/// A visitor that can be used to silently discard structured-field parts.
pub struct Ignored;

impl<'a> ParameterVisitor<'a> for Ignored {
    type Error = Infallible;

    fn parameter(
        &mut self,
        _key: &'a KeyRef,
        _value: BareItemFromInput<'a>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<'a> ItemVisitor<'a> for Ignored {
    type Error = Infallible;

    fn bare_item(
        self,
        _bare_item: BareItemFromInput<'a>,
    ) -> Result<impl ParameterVisitor<'a>, Self::Error> {
        Ok(Ignored)
    }
}

impl<'a> EntryVisitor<'a> for Ignored {
    fn inner_list(self) -> Result<impl InnerListVisitor<'a>, Self::Error> {
        Ok(Ignored)
    }
}

impl<'a> InnerListVisitor<'a> for Ignored {
    type Error = Infallible;

    fn item(&mut self) -> Result<impl ItemVisitor<'a>, Self::Error> {
        Ok(Ignored)
    }

    fn finish(self) -> Result<impl ParameterVisitor<'a>, Self::Error> {
        Ok(Ignored)
    }
}

impl<'a> DictionaryVisitor<'a> for Ignored {
    type Error = Infallible;

    fn entry(&mut self, _key: &'a KeyRef) -> Result<impl EntryVisitor<'a>, Self::Error> {
        Ok(Ignored)
    }
}

impl<'a> ListVisitor<'a> for Ignored {
    type Error = Infallible;

    fn entry(&mut self) -> Result<impl EntryVisitor<'a>, Self::Error> {
        Ok(Ignored)
    }
}
