use crate::serializer::Serializer;
use crate::{Error, Item, KeyRef, ListEntry, RefBareItem, SFVResult};

use std::borrow::BorrowMut;

/// Serializes `Item` field value components incrementally.
/// ```
/// use sfv::{KeyRef, RefItemSerializer};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let serialized_item = RefItemSerializer::new()
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
pub struct RefItemSerializer<W> {
    buffer: W,
}

impl Default for RefItemSerializer<String> {
    fn default() -> Self {
        Self::new()
    }
}

impl RefItemSerializer<String> {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }
}

impl<'a> RefItemSerializer<&'a mut String> {
    pub fn with_buffer(buffer: &'a mut String) -> Self {
        Self { buffer }
    }
}

impl<W: BorrowMut<String>> RefItemSerializer<W> {
    pub fn bare_item<'b>(
        mut self,
        bare_item: impl Into<RefBareItem<'b>>,
    ) -> RefParameterSerializer<W> {
        Serializer::serialize_bare_item(bare_item, self.buffer.borrow_mut());
        RefParameterSerializer {
            buffer: self.buffer,
        }
    }
}

/// Used by `RefItemSerializer`, `RefListSerializer`, `RefDictSerializer` to serialize a single `Parameter`.
#[derive(Debug)]
pub struct RefParameterSerializer<W> {
    buffer: W,
}

impl<W: BorrowMut<String>> RefParameterSerializer<W> {
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
/// use sfv::{KeyRef, StringRef, TokenRef, RefListSerializer};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut ser = RefListSerializer::new();
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
pub struct RefListSerializer<W> {
    buffer: W,
    first: bool,
}

impl Default for RefListSerializer<String> {
    fn default() -> Self {
        Self::new()
    }
}

impl RefListSerializer<String> {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            first: true,
        }
    }
}

impl<'a> RefListSerializer<&'a mut String> {
    pub fn with_buffer(buffer: &'a mut String) -> Self {
        Self {
            buffer,
            first: true,
        }
    }
}

impl<W: BorrowMut<String>> RefListSerializer<W> {
    pub fn bare_item<'b>(
        &mut self,
        bare_item: impl Into<RefBareItem<'b>>,
    ) -> RefParameterSerializer<&mut String> {
        let buffer = self.buffer.borrow_mut();
        maybe_write_separator(buffer, &mut self.first);
        Serializer::serialize_bare_item(bare_item, buffer);
        RefParameterSerializer { buffer }
    }

    pub fn inner_list(&mut self) -> RefInnerListSerializer {
        let buffer = self.buffer.borrow_mut();
        maybe_write_separator(buffer, &mut self.first);
        buffer.push('(');
        RefInnerListSerializer {
            buffer: Some(buffer),
        }
    }

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

    pub fn finish(self) -> SFVResult<W> {
        if self.first {
            return Err(Error::new("serializing empty list is not allowed"));
        }
        Ok(self.buffer)
    }
}

/// Serializes `Dictionary` field value components incrementally.
/// ```
/// use sfv::{KeyRef, StringRef, TokenRef, RefDictSerializer, Decimal};
/// use std::convert::TryFrom;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut ser = RefDictSerializer::new();
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
pub struct RefDictSerializer<W> {
    buffer: W,
    first: bool,
}

impl Default for RefDictSerializer<String> {
    fn default() -> Self {
        Self::new()
    }
}

impl RefDictSerializer<String> {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            first: true,
        }
    }
}

impl<'a> RefDictSerializer<&'a mut String> {
    pub fn with_buffer(buffer: &'a mut String) -> Self {
        Self {
            buffer,
            first: true,
        }
    }
}

impl<W: BorrowMut<String>> RefDictSerializer<W> {
    pub fn bare_item<'b>(
        &mut self,
        name: &KeyRef,
        value: impl Into<RefBareItem<'b>>,
    ) -> RefParameterSerializer<&mut String> {
        let buffer = self.buffer.borrow_mut();
        maybe_write_separator(buffer, &mut self.first);
        Serializer::serialize_key(name, buffer);
        let value = value.into();
        if value != RefBareItem::Boolean(true) {
            buffer.push('=');
            Serializer::serialize_bare_item(value, buffer);
        }
        RefParameterSerializer { buffer }
    }

    pub fn inner_list(&mut self, name: &KeyRef) -> RefInnerListSerializer {
        let buffer = self.buffer.borrow_mut();
        maybe_write_separator(buffer, &mut self.first);
        Serializer::serialize_key(name, buffer);
        buffer.push_str("=(");
        RefInnerListSerializer {
            buffer: Some(buffer),
        }
    }

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

    pub fn finish(self) -> SFVResult<W> {
        if self.first {
            return Err(Error::new("serializing empty dictionary is not allowed"));
        }
        Ok(self.buffer)
    }
}

/// Used by `RefItemSerializer`, `RefListSerializer`, `RefDictSerializer` to serialize `InnerList`.
// https://httpwg.org/specs/rfc8941.html#ser-innerlist
#[derive(Debug)]
pub struct RefInnerListSerializer<'a> {
    buffer: Option<&'a mut String>,
}

impl Drop for RefInnerListSerializer<'_> {
    fn drop(&mut self) {
        if let Some(ref mut buffer) = self.buffer {
            buffer.push(')');
        }
    }
}

impl<'a> RefInnerListSerializer<'a> {
    pub fn bare_item<'b>(
        &mut self,
        bare_item: impl Into<RefBareItem<'b>>,
    ) -> RefParameterSerializer<&mut String> {
        let buffer = self.buffer.as_mut().unwrap();
        if !buffer.is_empty() & !buffer.ends_with('(') {
            buffer.push(' ');
        }
        Serializer::serialize_bare_item(bare_item, buffer);
        RefParameterSerializer { buffer }
    }

    pub fn items<'b>(&mut self, items: impl IntoIterator<Item = &'b Item>) {
        for item in items {
            self.bare_item(&item.bare_item).parameters(&item.params);
        }
    }

    pub fn finish(mut self) -> RefParameterSerializer<&'a mut String> {
        let buffer = self.buffer.take().unwrap();
        buffer.push(')');
        RefParameterSerializer { buffer }
    }
}

#[cfg(test)]
mod alternative_serializer_tests {
    use super::*;
    use crate::{key_ref, string_ref, token_ref, Decimal};
    use std::convert::TryFrom;

    #[test]
    fn test_fast_serialize_item() {
        fn check(ser: RefItemSerializer<impl BorrowMut<String>>) {
            let output = ser
                .bare_item(token_ref("hello"))
                .parameter(key_ref("abc"), true)
                .finish();
            assert_eq!("hello;abc", output.borrow());
        }

        check(RefItemSerializer::new());
        check(RefItemSerializer::with_buffer(&mut String::new()));
    }

    #[test]
    fn test_fast_serialize_list() -> SFVResult<()> {
        fn check(mut ser: RefListSerializer<impl BorrowMut<String>>) -> SFVResult<()> {
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

        check(RefListSerializer::new())?;
        check(RefListSerializer::with_buffer(&mut String::new()))?;
        Ok(())
    }

    #[test]
    fn test_fast_serialize_dict() -> SFVResult<()> {
        fn check(mut ser: RefDictSerializer<impl BorrowMut<String>>) -> SFVResult<()> {
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

        check(RefDictSerializer::new())?;
        check(RefDictSerializer::with_buffer(&mut String::new()))?;
        Ok(())
    }

    #[test]
    fn test_serialize_empty() {
        assert!(RefListSerializer::new().finish().is_err());
        assert!(RefDictSerializer::new().finish().is_err());

        let mut output = String::from(" ");
        assert!(RefListSerializer::with_buffer(&mut output)
            .finish()
            .is_err());

        let mut output = String::from(" ");
        assert!(RefDictSerializer::with_buffer(&mut output)
            .finish()
            .is_err());
    }

    // Regression test for https://github.com/undef1nd/sfv/issues/131.
    #[test]
    fn test_with_buffer_separator() -> SFVResult<()> {
        let mut output = String::from(" ");
        RefListSerializer::with_buffer(&mut output).bare_item(1);
        assert_eq!(output, " 1");

        let mut output = String::from(" ");
        RefDictSerializer::with_buffer(&mut output).bare_item(key_ref("key1"), 1);
        assert_eq!(output, " key1=1");

        Ok(())
    }
}
