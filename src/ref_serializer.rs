use crate::serializer::Serializer;
use crate::{Error, KeyRef, RefBareItem, SFVResult};

use std::borrow::BorrowMut;

/// Serializes `Item` field value components incrementally.
/// ```
/// use sfv::{KeyRef, ItemSerializer};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let serialized_item = ItemSerializer::new()
///     .bare_item(11)
///     .parameter(KeyRef::from_str("foo")?, true)
///     .finish();
///
/// assert_eq!(serialized_item, "11;foo");
/// # Ok(())
/// # }
/// ```
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

/// Used by `ItemSerializer`, `ListSerializer`, `DictSerializer` to serialize a single `Parameter`.
#[derive(Debug)]
pub struct ParameterSerializer<T> {
    buffer: T,
}

impl<T: BufferHolder> ParameterSerializer<T> {
    pub fn parameter<'b>(mut self, name: &KeyRef, value: impl Into<RefBareItem<'b>>) -> Self {
        Serializer::serialize_parameter(name, value, self.buffer.get_mut());
        self
    }

    pub fn finish(self) -> T {
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
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let serialized_list = ListSerializer::new()
///     .bare_item(11)
///     .parameter(KeyRef::from_str("foo")?, true)?
///     .open_inner_list()
///     .inner_list_bare_item(TokenRef::from_str("abc")?)
///     .inner_list_parameter(KeyRef::from_str("abc_param")?, false)?
///     .inner_list_bare_item(TokenRef::from_str("def")?)
///     .close_inner_list()
///     .parameter(KeyRef::from_str("bar")?, StringRef::from_str("val")?)?
///     .finish()?;
/// assert_eq!(
///     serialized_list,
///     r#"11;foo, (abc;abc_param=?0 def);bar="val""#
/// );
/// # Ok(())
/// # }
/// ```
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
    pub fn bare_item<'b>(mut self, bare_item: impl Into<RefBareItem<'b>>) -> Self {
        maybe_write_separator(self.buffer.borrow_mut(), &mut self.first);
        Serializer::serialize_bare_item(bare_item, self.buffer.borrow_mut());
        self
    }

    pub fn parameter<'b>(
        mut self,
        name: &KeyRef,
        value: impl Into<RefBareItem<'b>>,
    ) -> SFVResult<Self> {
        if self.first {
            return Err(Error::new(
                "parameters must be serialized after bare item or inner list",
            ));
        }
        Serializer::serialize_parameter(name, value, self.buffer.borrow_mut());
        Ok(self)
    }

    pub fn open_inner_list(mut self) -> InnerListSerializer<Self> {
        maybe_write_separator(self.buffer.borrow_mut(), &mut self.first);
        self.buffer.borrow_mut().push('(');
        InnerListSerializer { buffer: self }
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
/// use sfv::{KeyRef, StringRef, TokenRef, DictSerializer, Decimal};
/// use std::convert::TryFrom;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let serialized_dict = DictSerializer::new()
///    .bare_item_member(KeyRef::from_str("member1")?, 11)
///    .parameter(KeyRef::from_str("foo")?, true)?
///    .open_inner_list(KeyRef::from_str("member2")?)
///    .inner_list_bare_item(TokenRef::from_str("abc")?)
///    .inner_list_parameter(KeyRef::from_str("abc_param")?, false)?
///    .inner_list_bare_item(TokenRef::from_str("def")?)
///    .close_inner_list()
///    .parameter(KeyRef::from_str("bar")?, StringRef::from_str("val")?)?
///    .bare_item_member(KeyRef::from_str("member3")?, Decimal::try_from(12.34566)?)
///    .finish()?;
/// assert_eq!(
///    serialized_dict,
///    r#"member1=11;foo, member2=(abc;abc_param=?0 def);bar="val", member3=12.346"#
/// );
/// # Ok(())
/// # }
/// ```
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
    pub fn bare_item_member<'b>(
        mut self,
        name: &KeyRef,
        value: impl Into<RefBareItem<'b>>,
    ) -> Self {
        maybe_write_separator(self.buffer.borrow_mut(), &mut self.first);
        Serializer::serialize_key(name, self.buffer.borrow_mut());
        let value = value.into();
        if value != RefBareItem::Boolean(true) {
            self.buffer.borrow_mut().push('=');
            Serializer::serialize_bare_item(value, self.buffer.borrow_mut());
        }
        self
    }

    pub fn parameter<'b>(
        mut self,
        name: &KeyRef,
        value: impl Into<RefBareItem<'b>>,
    ) -> SFVResult<Self> {
        if self.first {
            return Err(Error::new(
                "parameters must be serialized after bare item or inner list",
            ));
        }
        Serializer::serialize_parameter(name, value, self.buffer.borrow_mut());
        Ok(self)
    }

    pub fn open_inner_list(mut self, name: &KeyRef) -> InnerListSerializer<Self> {
        maybe_write_separator(self.buffer.borrow_mut(), &mut self.first);
        Serializer::serialize_key(name, self.buffer.borrow_mut());
        self.buffer.borrow_mut().push_str("=(");
        InnerListSerializer { buffer: self }
    }

    pub fn finish(self) -> SFVResult<W> {
        if self.first {
            return Err(Error::new("serializing empty dictionary is not allowed"));
        }
        Ok(self.buffer)
    }
}

/// Used by `ItemSerializer`, `ListSerializer`, `DictSerializer` to serialize `InnerList`.
#[derive(Debug)]
pub struct InnerListSerializer<T> {
    buffer: T,
}

impl<T: BufferHolder> InnerListSerializer<T> {
    pub fn inner_list_bare_item<'b>(mut self, bare_item: impl Into<RefBareItem<'b>>) -> Self {
        let buffer = self.buffer.get_mut();
        if !buffer.is_empty() & !buffer.ends_with('(') {
            buffer.push(' ');
        }
        Serializer::serialize_bare_item(bare_item, buffer);
        self
    }

    pub fn inner_list_parameter<'b>(
        mut self,
        name: &KeyRef,
        value: impl Into<RefBareItem<'b>>,
    ) -> SFVResult<Self> {
        let buffer = self.buffer.get_mut();
        if buffer.is_empty() {
            return Err(Error::new(
                "parameters must be serialized after bare item or inner list",
            ));
        }
        Serializer::serialize_parameter(name, value, buffer);
        Ok(self)
    }

    pub fn close_inner_list(mut self) -> T {
        self.buffer.get_mut().push(')');
        self.buffer
    }
}

pub trait BufferHolder {
    fn get_mut(&mut self) -> &mut String;
}

impl<W: BorrowMut<String>> BufferHolder for ListSerializer<W> {
    fn get_mut(&mut self) -> &mut String {
        self.buffer.borrow_mut()
    }
}

impl<W: BorrowMut<String>> BufferHolder for DictSerializer<W> {
    fn get_mut(&mut self) -> &mut String {
        self.buffer.borrow_mut()
    }
}

impl BufferHolder for String {
    fn get_mut(&mut self) -> &mut String {
        self
    }
}

#[cfg(test)]
mod alternative_serializer_tests {
    use super::*;
    use crate::{key_ref, string_ref, token_ref, Decimal};
    use std::convert::TryFrom;

    #[test]
    fn test_fast_serialize_item() -> SFVResult<()> {
        let output = ItemSerializer::new()
            .bare_item(token_ref("hello"))
            .parameter(key_ref("abc"), true)
            .finish();
        assert_eq!("hello;abc", output);
        Ok(())
    }

    #[test]
    fn test_fast_serialize_list() -> SFVResult<()> {
        let output = ListSerializer::new()
            .bare_item(token_ref("hello"))
            .parameter(key_ref("key1"), true)?
            .parameter(key_ref("key2"), false)?
            .open_inner_list()
            .inner_list_bare_item(string_ref("some_string"))
            .inner_list_bare_item(12)
            .inner_list_parameter(key_ref("inner-member-key"), true)?
            .close_inner_list()
            .parameter(key_ref("inner-list-param"), token_ref("*"))?
            .finish()?;
        assert_eq!(
            "hello;key1;key2=?0, (\"some_string\" 12;inner-member-key);inner-list-param=*",
            output
        );
        Ok(())
    }

    #[test]
    fn test_fast_serialize_dict() -> SFVResult<()> {
        let output = DictSerializer::new()
            .bare_item_member(key_ref("member1"), token_ref("hello"))
            .parameter(key_ref("key1"), true)?
            .parameter(key_ref("key2"), false)?
            .bare_item_member(key_ref("member2"), true)
            .parameter(key_ref("key3"), Decimal::try_from(45.4586).unwrap())?
            .parameter(key_ref("key4"), string_ref("str"))?
            .open_inner_list(key_ref("key5"))
            .inner_list_bare_item(45)
            .inner_list_bare_item(0)
            .close_inner_list()
            .bare_item_member(key_ref("key6"), string_ref("foo"))
            .open_inner_list(key_ref("key7"))
            .inner_list_bare_item("some_string".as_bytes())
            .inner_list_bare_item("other_string".as_bytes())
            .close_inner_list()
            .parameter(key_ref("lparam"), 10)?
            .bare_item_member(key_ref("key8"), true)
            .finish()?;
        assert_eq!(
            "member1=hello;key1;key2=?0, member2;key3=45.459;key4=\"str\", key5=(45 0), key6=\"foo\", key7=(:c29tZV9zdHJpbmc=: :b3RoZXJfc3RyaW5n:);lparam=10, key8",
            output
        );
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
        DictSerializer::with_buffer(&mut output).bare_item_member(key_ref("key1"), 1);
        assert_eq!(output, " key1=1");

        Ok(())
    }
}
