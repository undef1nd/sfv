use crate::serializer::Serializer;
use crate::{Error, KeyRef, RefBareItem, SFVResult};

#[cfg(feature = "parsed-types")]
use crate::{Item, ListEntry};

use std::borrow::BorrowMut;

/// Serializes `Item` field value components incrementally.
/// ```
/// use sfv::{KeyRef, ItemSerializer};
///
/// # fn main() -> Result<(), sfv::Error> {
/// let serialized_item = ItemSerializer::new()
///     .bare_item(11)
///     .parameter(KeyRef::from_str("foo")?, true)
///     .finish();
///
/// assert_eq!(serialized_item, "11;foo");
/// # Ok(())
/// # }
/// ```
// https://httpwg.org/specs/rfc8941.html#ser-item
#[derive(Debug)]
pub struct ItemSerializer<W> {
    buffer: W,
}

impl Default for ItemSerializer<String> {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemSerializer<String> {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }
}

impl<'a> ItemSerializer<&'a mut String> {
    pub fn with_buffer(buffer: &'a mut String) -> Self {
        Self { buffer }
    }
}

impl<W: BorrowMut<String>> ItemSerializer<W> {
    pub fn bare_item<'b>(
        mut self,
        bare_item: impl Into<RefBareItem<'b>>,
    ) -> ParameterSerializer<W> {
        Serializer::serialize_bare_item(bare_item, self.buffer.borrow_mut());
        ParameterSerializer {
            buffer: self.buffer,
        }
    }
}

/// Serializes parameters incrementally.
#[derive(Debug)]
pub struct ParameterSerializer<W> {
    buffer: W,
}

impl<W: BorrowMut<String>> ParameterSerializer<W> {
    pub fn parameter<'b>(mut self, name: &KeyRef, value: impl Into<RefBareItem<'b>>) -> Self {
        Serializer::serialize_parameter(name, value, self.buffer.borrow_mut());
        self
    }

    pub fn parameters<'b>(
        mut self,
        params: impl IntoIterator<Item = (impl AsRef<KeyRef>, impl Into<RefBareItem<'b>>)>,
    ) -> Self {
        for (name, value) in params {
            Serializer::serialize_parameter(name.as_ref(), value, self.buffer.borrow_mut());
        }
        self
    }

    pub fn finish(self) -> W {
        self.buffer
    }
}

fn maybe_write_separator(buffer: &mut String, first: &mut bool) {
    if *first {
        *first = false;
    } else {
        buffer.push_str(", ");
    }
}

/// Serializes `List` field value components incrementally.
/// ```
/// use sfv::{KeyRef, StringRef, TokenRef, ListSerializer};
///
/// # fn main() -> Result<(), sfv::Error> {
/// let mut ser = ListSerializer::new();
///
/// ser.bare_item(11)
///     .parameter(KeyRef::from_str("foo")?, true);
///
/// {
///     let mut ser = ser.inner_list();
///
///     ser.bare_item(TokenRef::from_str("abc")?)
///         .parameter(KeyRef::from_str("abc_param")?, false);
///
///     ser.bare_item(TokenRef::from_str("def")?);
///
///     ser.finish()
///         .parameter(KeyRef::from_str("bar")?, StringRef::from_str("val")?);
/// }
///
/// assert_eq!(
///     ser.finish()?,
///     r#"11;foo, (abc;abc_param=?0 def);bar="val""#
/// );
/// # Ok(())
/// # }
/// ```
// https://httpwg.org/specs/rfc8941.html#ser-list
#[derive(Debug)]
pub struct ListSerializer<W> {
    buffer: W,
    first: bool,
}

impl Default for ListSerializer<String> {
    fn default() -> Self {
        Self::new()
    }
}

impl ListSerializer<String> {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            first: true,
        }
    }
}

impl<'a> ListSerializer<&'a mut String> {
    pub fn with_buffer(buffer: &'a mut String) -> Self {
        Self {
            buffer,
            first: true,
        }
    }
}

impl<W: BorrowMut<String>> ListSerializer<W> {
    pub fn bare_item<'b>(
        &mut self,
        bare_item: impl Into<RefBareItem<'b>>,
    ) -> ParameterSerializer<&mut String> {
        let buffer = self.buffer.borrow_mut();
        maybe_write_separator(buffer, &mut self.first);
        Serializer::serialize_bare_item(bare_item, buffer);
        ParameterSerializer { buffer }
    }

    pub fn inner_list(&mut self) -> InnerListSerializer {
        let buffer = self.buffer.borrow_mut();
        maybe_write_separator(buffer, &mut self.first);
        buffer.push('(');
        InnerListSerializer {
            buffer: Some(buffer),
        }
    }

    #[cfg(feature = "parsed-types")]
    pub fn members<'b>(&mut self, members: impl IntoIterator<Item = &'b ListEntry>) {
        for value in members {
            match value {
                ListEntry::Item(value) => {
                    self.bare_item(&value.bare_item).parameters(&value.params);
                }
                ListEntry::InnerList(value) => {
                    let mut ser = self.inner_list();
                    ser.items(&value.items);
                    ser.finish().parameters(&value.params);
                }
            }
        }
    }

    /// Finishes serialization of the list and returns the underlying output.
    ///
    /// This can only fail if no members were serialized, as [empty lists are
    /// not meant to be serialized at
    /// all](https://httpwg.org/specs/rfc8941.html#text-serialize).
    pub fn finish(self) -> SFVResult<W> {
        if self.first {
            return Err(Error::new("serializing empty list is not allowed"));
        }
        Ok(self.buffer)
    }
}

/// Serializes `Dictionary` field value components incrementally.
/// ```
/// use sfv::{KeyRef, StringRef, TokenRef, DictSerializer, Decimal};
/// use std::convert::TryFrom;
///
/// # fn main() -> Result<(), sfv::Error> {
/// let mut ser = DictSerializer::new();
///
/// ser.bare_item(KeyRef::from_str("member1")?, 11)
///     .parameter(KeyRef::from_str("foo")?, true);
///
/// {
///   let mut ser = ser.inner_list(KeyRef::from_str("member2")?);
///
///   ser.bare_item(TokenRef::from_str("abc")?)
///       .parameter(KeyRef::from_str("abc_param")?, false);
///
///   ser.bare_item(TokenRef::from_str("def")?);
///
///   ser.finish()
///      .parameter(KeyRef::from_str("bar")?, StringRef::from_str("val")?);
/// }
///
/// ser.bare_item(KeyRef::from_str("member3")?, Decimal::try_from(12.34566)?);
///
/// assert_eq!(
///     ser.finish()?,
///     r#"member1=11;foo, member2=(abc;abc_param=?0 def);bar="val", member3=12.346"#
/// );
/// # Ok(())
/// # }
/// ```
// https://httpwg.org/specs/rfc8941.html#ser-dictionary
#[derive(Debug)]
pub struct DictSerializer<W> {
    buffer: W,
    first: bool,
}

impl Default for DictSerializer<String> {
    fn default() -> Self {
        Self::new()
    }
}

impl DictSerializer<String> {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            first: true,
        }
    }
}

impl<'a> DictSerializer<&'a mut String> {
    pub fn with_buffer(buffer: &'a mut String) -> Self {
        Self {
            buffer,
            first: true,
        }
    }
}

impl<W: BorrowMut<String>> DictSerializer<W> {
    pub fn bare_item<'b>(
        &mut self,
        name: &KeyRef,
        value: impl Into<RefBareItem<'b>>,
    ) -> ParameterSerializer<&mut String> {
        let buffer = self.buffer.borrow_mut();
        maybe_write_separator(buffer, &mut self.first);
        Serializer::serialize_key(name, buffer);
        let value = value.into();
        if value != RefBareItem::Boolean(true) {
            buffer.push('=');
            Serializer::serialize_bare_item(value, buffer);
        }
        ParameterSerializer { buffer }
    }

    pub fn inner_list(&mut self, name: &KeyRef) -> InnerListSerializer {
        let buffer = self.buffer.borrow_mut();
        maybe_write_separator(buffer, &mut self.first);
        Serializer::serialize_key(name, buffer);
        buffer.push_str("=(");
        InnerListSerializer {
            buffer: Some(buffer),
        }
    }

    #[cfg(feature = "parsed-types")]
    pub fn members<'b>(
        &mut self,
        members: impl IntoIterator<Item = (impl AsRef<KeyRef>, &'b ListEntry)>,
    ) {
        for (name, value) in members {
            match value {
                ListEntry::Item(value) => {
                    self.bare_item(name.as_ref(), &value.bare_item)
                        .parameters(&value.params);
                }
                ListEntry::InnerList(value) => {
                    let mut ser = self.inner_list(name.as_ref());
                    ser.items(&value.items);
                    ser.finish().parameters(&value.params);
                }
            }
        }
    }

    /// Finishes serialization of the dictionary and returns the underlying output.
    ///
    /// This can only fail if no members were serialized, as [empty dictionaries
    /// are not meant to be serialized at
    /// all](https://httpwg.org/specs/rfc8941.html#text-serialize).
    pub fn finish(self) -> SFVResult<W> {
        if self.first {
            return Err(Error::new("serializing empty dictionary is not allowed"));
        }
        Ok(self.buffer)
    }
}

/// Serializes inner lists incrementally.
// https://httpwg.org/specs/rfc8941.html#ser-innerlist
#[derive(Debug)]
pub struct InnerListSerializer<'a> {
    buffer: Option<&'a mut String>,
}

impl Drop for InnerListSerializer<'_> {
    fn drop(&mut self) {
        if let Some(ref mut buffer) = self.buffer {
            buffer.push(')');
        }
    }
}

impl<'a> InnerListSerializer<'a> {
    pub fn bare_item<'b>(
        &mut self,
        bare_item: impl Into<RefBareItem<'b>>,
    ) -> ParameterSerializer<&mut String> {
        let buffer = self.buffer.as_mut().unwrap();
        if !buffer.is_empty() & !buffer.ends_with('(') {
            buffer.push(' ');
        }
        Serializer::serialize_bare_item(bare_item, buffer);
        ParameterSerializer { buffer }
    }

    #[cfg(feature = "parsed-types")]
    pub fn items<'b>(&mut self, items: impl IntoIterator<Item = &'b Item>) {
        for item in items {
            self.bare_item(&item.bare_item).parameters(&item.params);
        }
    }

    pub fn finish(mut self) -> ParameterSerializer<&'a mut String> {
        let buffer = self.buffer.take().unwrap();
        buffer.push(')');
        ParameterSerializer { buffer }
    }
}

#[cfg(test)]
mod alternative_serializer_tests {
    use super::*;
    use crate::{key_ref, string_ref, token_ref, Decimal};
    use std::convert::TryFrom;

    #[test]
    fn test_fast_serialize_item() {
        fn check(ser: ItemSerializer<impl BorrowMut<String>>) {
            let output = ser
                .bare_item(token_ref("hello"))
                .parameter(key_ref("abc"), true)
                .finish();
            assert_eq!("hello;abc", output.borrow());
        }

        check(ItemSerializer::new());
        check(ItemSerializer::with_buffer(&mut String::new()));
    }

    #[test]
    fn test_fast_serialize_list() -> SFVResult<()> {
        fn check(mut ser: ListSerializer<impl BorrowMut<String>>) -> SFVResult<()> {
            ser.bare_item(token_ref("hello"))
                .parameter(key_ref("key1"), true)
                .parameter(key_ref("key2"), false);

            {
                let mut ser = ser.inner_list();
                ser.bare_item(string_ref("some_string"));
                ser.bare_item(12)
                    .parameter(key_ref("inner-member-key"), true);
                ser.finish()
                    .parameter(key_ref("inner-list-param"), token_ref("*"));
            }

            let output = ser.finish()?;
            assert_eq!(
                "hello;key1;key2=?0, (\"some_string\" 12;inner-member-key);inner-list-param=*",
                output.borrow()
            );
            Ok(())
        }

        check(ListSerializer::new())?;
        check(ListSerializer::with_buffer(&mut String::new()))?;
        Ok(())
    }

    #[test]
    fn test_fast_serialize_dict() -> SFVResult<()> {
        fn check(mut ser: DictSerializer<impl BorrowMut<String>>) -> SFVResult<()> {
            ser.bare_item(key_ref("member1"), token_ref("hello"))
                .parameter(key_ref("key1"), true)
                .parameter(key_ref("key2"), false);

            ser.bare_item(key_ref("member2"), true)
                .parameter(key_ref("key3"), Decimal::try_from(45.4586).unwrap())
                .parameter(key_ref("key4"), string_ref("str"));

            {
                let mut ser = ser.inner_list(key_ref("key5"));
                ser.bare_item(45);
                ser.bare_item(0);
            }

            ser.bare_item(key_ref("key6"), string_ref("foo"));

            {
                let mut ser = ser.inner_list(key_ref("key7"));
                ser.bare_item("some_string".as_bytes());
                ser.bare_item("other_string".as_bytes());
                ser.finish().parameter(key_ref("lparam"), 10);
            }

            ser.bare_item(key_ref("key8"), true);

            let output = ser.finish()?;
            assert_eq!(
                "member1=hello;key1;key2=?0, member2;key3=45.459;key4=\"str\", key5=(45 0), key6=\"foo\", key7=(:c29tZV9zdHJpbmc=: :b3RoZXJfc3RyaW5n:);lparam=10, key8",
                output.borrow()
            );
            Ok(())
        }

        check(DictSerializer::new())?;
        check(DictSerializer::with_buffer(&mut String::new()))?;
        Ok(())
    }

    #[test]
    fn test_serialize_empty() {
        assert!(ListSerializer::new().finish().is_err());
        assert!(DictSerializer::new().finish().is_err());

        let mut output = String::from(" ");
        assert!(ListSerializer::with_buffer(&mut output).finish().is_err());

        let mut output = String::from(" ");
        assert!(DictSerializer::with_buffer(&mut output).finish().is_err());
    }

    // Regression test for https://github.com/undef1nd/sfv/issues/131.
    #[test]
    fn test_with_buffer_separator() -> SFVResult<()> {
        let mut output = String::from(" ");
        ListSerializer::with_buffer(&mut output).bare_item(1);
        assert_eq!(output, " 1");

        let mut output = String::from(" ");
        DictSerializer::with_buffer(&mut output).bare_item(key_ref("key1"), 1);
        assert_eq!(output, " key1=1");

        Ok(())
    }
}
