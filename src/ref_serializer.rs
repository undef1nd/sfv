use crate::serializer::Serializer;
use crate::{AsRefBareItem, Error, RefBareItem, SFVResult};

use std::borrow::BorrowMut;

/// Serializes `Item` field value components incrementally.
/// ```
/// use sfv::RefItemSerializer;
///
/// # fn main() -> Result<(), sfv::Error> {
/// let serialized_item = RefItemSerializer::new()
///     .bare_item(11)?
///     .parameter("foo", true)?
///     .finish();
///
/// assert_eq!(serialized_item, "11;foo");
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct RefItemSerializer<W> {
    buffer: W,
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
    pub fn bare_item(
        mut self,
        bare_item: impl AsRefBareItem,
    ) -> SFVResult<RefParameterSerializer<W>> {
        Serializer::serialize_bare_item(bare_item, self.buffer.borrow_mut())?;
        Ok(RefParameterSerializer {
            buffer: self.buffer,
        })
    }
}

/// Used by `RefItemSerializer`, `RefListSerializer`, `RefDictSerializer` to serialize a single `Parameter`.
#[derive(Debug)]
pub struct RefParameterSerializer<T> {
    buffer: T,
}

impl<T: BufferHolder> RefParameterSerializer<T> {
    pub fn parameter(mut self, name: &str, value: impl AsRefBareItem) -> SFVResult<Self> {
        Serializer::serialize_parameter(name, value, self.buffer.get_mut())?;
        Ok(self)
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
/// use sfv::{RefBareItem, RefListSerializer};
///
/// # fn main() -> Result<(), sfv::Error> {
/// let serialized_list = RefListSerializer::new()
///     .bare_item(11)?
///     .parameter("foo", true)?
///     .open_inner_list()
///     .inner_list_bare_item(RefBareItem::Token("abc"))?
///     .inner_list_parameter("abc_param", false)?
///     .inner_list_bare_item(RefBareItem::Token("def"))?
///     .close_inner_list()
///     .parameter("bar", RefBareItem::String("val"))?
///     .finish()?;
///
/// assert_eq!(
///     serialized_list,
///     r#"11;foo, (abc;abc_param=?0 def);bar="val""#
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct RefListSerializer<W> {
    buffer: W,
    first: bool,
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
    pub fn bare_item(mut self, bare_item: impl AsRefBareItem) -> SFVResult<Self> {
        maybe_write_separator(self.buffer.borrow_mut(), &mut self.first);
        Serializer::serialize_bare_item(bare_item, self.buffer.borrow_mut())?;
        Ok(self)
    }

    pub fn parameter(mut self, name: &str, value: impl AsRefBareItem) -> SFVResult<Self> {
        if self.first {
            return Err(Error::new(
                "parameters must be serialized after bare item or inner list",
            ));
        }
        Serializer::serialize_parameter(name, value, self.buffer.borrow_mut())?;
        Ok(self)
    }

    pub fn open_inner_list(mut self) -> RefInnerListSerializer<Self> {
        maybe_write_separator(self.buffer.borrow_mut(), &mut self.first);
        self.buffer.borrow_mut().push('(');
        RefInnerListSerializer { buffer: self }
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
/// use sfv::{Decimal, FromPrimitive, RefBareItem, RefDictSerializer};
///
/// # fn main() -> Result<(), sfv::Error> {
/// let serialized_dict = RefDictSerializer::new()
///    .bare_item_member("member1", 11)?
///    .parameter("foo", true)?
///    .open_inner_list("member2")?
///    .inner_list_bare_item(RefBareItem::Token("abc"))?
///    .inner_list_parameter("abc_param", false)?
///    .inner_list_bare_item(RefBareItem::Token("def"))?
///    .close_inner_list()
///    .parameter("bar", RefBareItem::String("val"))?
///    .bare_item_member(
///         "member3",
///         Decimal::from_f64(12.34566).unwrap(),
///    )?
///    .finish()?;
///
/// assert_eq!(
///    serialized_dict,
///    r#"member1=11;foo, member2=(abc;abc_param=?0 def);bar="val", member3=12.346"#
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct RefDictSerializer<W> {
    buffer: W,
    first: bool,
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
    pub fn bare_item_member(mut self, name: &str, value: impl AsRefBareItem) -> SFVResult<Self> {
        maybe_write_separator(self.buffer.borrow_mut(), &mut self.first);
        Serializer::serialize_key(name, self.buffer.borrow_mut())?;
        let value = value.as_ref_bare_item();
        if value != RefBareItem::Boolean(true) {
            self.buffer.borrow_mut().push('=');
            Serializer::serialize_bare_item(value, self.buffer.borrow_mut())?;
        }
        Ok(self)
    }

    pub fn parameter(mut self, name: &str, value: impl AsRefBareItem) -> SFVResult<Self> {
        if self.first {
            return Err(Error::new(
                "parameters must be serialized after bare item or inner list",
            ));
        }
        Serializer::serialize_parameter(name, value, self.buffer.borrow_mut())?;
        Ok(self)
    }

    pub fn open_inner_list(mut self, name: &str) -> SFVResult<RefInnerListSerializer<Self>> {
        maybe_write_separator(self.buffer.borrow_mut(), &mut self.first);
        Serializer::serialize_key(name, self.buffer.borrow_mut())?;
        self.buffer.borrow_mut().push_str("=(");
        Ok(RefInnerListSerializer { buffer: self })
    }

    pub fn finish(self) -> SFVResult<W> {
        if self.first {
            return Err(Error::new("serializing empty dictionary is not allowed"));
        }
        Ok(self.buffer)
    }
}

/// Used by `RefItemSerializer`, `RefListSerializer`, `RefDictSerializer` to serialize `InnerList`.
#[derive(Debug)]
pub struct RefInnerListSerializer<T> {
    buffer: T,
}

impl<T: BufferHolder> RefInnerListSerializer<T> {
    pub fn inner_list_bare_item(mut self, bare_item: impl AsRefBareItem) -> SFVResult<Self> {
        let buffer = self.buffer.get_mut();
        if !buffer.is_empty() & !buffer.ends_with('(') {
            buffer.push(' ');
        }
        Serializer::serialize_bare_item(bare_item, buffer)?;
        Ok(self)
    }

    pub fn inner_list_parameter(
        mut self,
        name: &str,
        value: impl AsRefBareItem,
    ) -> SFVResult<Self> {
        let buffer = self.buffer.get_mut();
        if buffer.is_empty() {
            return Err(Error::new(
                "parameters must be serialized after bare item or inner list",
            ));
        }
        Serializer::serialize_parameter(name, value, buffer)?;
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

impl<W: BorrowMut<String>> BufferHolder for RefListSerializer<W> {
    fn get_mut(&mut self) -> &mut String {
        self.buffer.borrow_mut()
    }
}

impl<W: BorrowMut<String>> BufferHolder for RefDictSerializer<W> {
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
    use crate::{Decimal, FromPrimitive};

    #[test]
    fn test_fast_serialize_item() -> SFVResult<()> {
        let output = RefItemSerializer::new()
            .bare_item(RefBareItem::Token("hello"))?
            .parameter("abc", true)?
            .finish();
        assert_eq!("hello;abc", output);
        Ok(())
    }

    #[test]
    fn test_fast_serialize_list() -> SFVResult<()> {
        let output = RefListSerializer::new()
            .bare_item(RefBareItem::Token("hello"))?
            .parameter("key1", true)?
            .parameter("key2", false)?
            .open_inner_list()
            .inner_list_bare_item(RefBareItem::String("some_string"))?
            .inner_list_bare_item(12)?
            .inner_list_parameter("inner-member-key", true)?
            .close_inner_list()
            .parameter("inner-list-param", RefBareItem::Token("*"))?
            .finish()?;
        assert_eq!(
            "hello;key1;key2=?0, (\"some_string\" 12;inner-member-key);inner-list-param=*",
            output
        );
        Ok(())
    }

    #[test]
    fn test_fast_serialize_dict() -> SFVResult<()> {
        let output = RefDictSerializer::new()
            .bare_item_member("member1", RefBareItem::Token("hello"))?
            .parameter("key1", true)?
            .parameter("key2", false)?
            .bare_item_member("member2", true)?
            .parameter("key3", Decimal::from_f64(45.4586).unwrap())?
            .parameter("key4", RefBareItem::String("str"))?
            .open_inner_list("key5")?
            .inner_list_bare_item(45)?
            .inner_list_bare_item(0)?
            .close_inner_list()
            .bare_item_member("key6", RefBareItem::String("foo"))?
            .open_inner_list("key7")?
            .inner_list_bare_item("some_string".as_bytes())?
            .inner_list_bare_item("other_string".as_bytes())?
            .close_inner_list()
            .parameter("lparam", 10)?
            .bare_item_member("key8", true)?
            .finish()?;
        assert_eq!(
            "member1=hello;key1;key2=?0, member2;key3=45.459;key4=\"str\", key5=(45 0), key6=\"foo\", key7=(:c29tZV9zdHJpbmc=: :b3RoZXJfc3RyaW5n:);lparam=10, key8",
            output
        );
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
        RefListSerializer::with_buffer(&mut output).bare_item(1)?;
        assert_eq!(output, " 1");

        let mut output = String::from(" ");
        RefDictSerializer::with_buffer(&mut output).bare_item_member("key1", 1)?;
        assert_eq!(output, " key1=1");

        Ok(())
    }
}
