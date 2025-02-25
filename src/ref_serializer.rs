use crate::serializer::Serializer;
use crate::{AsRefBareItem, RefBareItem, SFVResult};
use std::marker::PhantomData;

/// Serializes `Item` field value components incrementally.
/// ```
/// use sfv::RefItemSerializer;
///
/// # fn main() -> Result<(), &'static str> {
/// let mut serialized_item = String::new();
///
/// RefItemSerializer::new(&mut serialized_item)
///   .bare_item(11)?
///   .parameter("foo", true)?;
///
/// assert_eq!(serialized_item, "11;foo");
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct RefItemSerializer<'a> {
    pub buffer: &'a mut String,
}

impl<'a> RefItemSerializer<'a> {
    pub fn new(buffer: &'a mut String) -> Self {
        RefItemSerializer { buffer }
    }

    pub fn bare_item(self, bare_item: impl AsRefBareItem) -> SFVResult<RefParameterSerializer<'a>> {
        Serializer::serialize_bare_item(bare_item, self.buffer)?;
        Ok(RefParameterSerializer {
            buffer: self.buffer,
        })
    }
}

/// Used by `RefItemSerializer`, `RefListSerializer`, `RefDictSerializer` to serialize a single `Parameter`.
#[derive(Debug)]
pub struct RefParameterSerializer<'a> {
    buffer: &'a mut String,
}

impl RefParameterSerializer<'_> {
    pub fn parameter(self, name: &str, value: impl AsRefBareItem) -> SFVResult<Self> {
        Serializer::serialize_parameter(name, value, self.buffer)?;
        Ok(self)
    }
}

/// Serializes `List` field value components incrementally.
/// ```
/// use sfv::{RefBareItem, RefListSerializer};
///
/// # fn main() -> Result<(), &'static str> {
/// let mut serialized_item = String::new();
///
/// RefListSerializer::new(&mut serialized_item)
///     .bare_item(11)?
///     .parameter("foo", true)?
///     .open_inner_list()
///     .inner_list_bare_item(RefBareItem::Token("abc"))?
///     .inner_list_parameter("abc_param", false)?
///     .inner_list_bare_item(RefBareItem::Token("def"))?
///     .close_inner_list()
///     .parameter("bar", RefBareItem::String("val"))?;
///
/// assert_eq!(
///     serialized_item,
///     "11;foo, (abc;abc_param=?0 def);bar=\"val\""
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct RefListSerializer<'a> {
    buffer: &'a mut String,
}

impl<'a> RefListSerializer<'a> {
    pub fn new(buffer: &'a mut String) -> Self {
        RefListSerializer { buffer }
    }

    pub fn bare_item(self, bare_item: impl AsRefBareItem) -> SFVResult<Self> {
        if !self.buffer.is_empty() {
            self.buffer.push_str(", ");
        }
        Serializer::serialize_bare_item(bare_item, self.buffer)?;
        Ok(RefListSerializer {
            buffer: self.buffer,
        })
    }

    pub fn parameter(self, name: &str, value: impl AsRefBareItem) -> SFVResult<Self> {
        if self.buffer.is_empty() {
            return Err("parameters must be serialized after bare item or inner list");
        }
        Serializer::serialize_parameter(name, value, self.buffer)?;
        Ok(RefListSerializer {
            buffer: self.buffer,
        })
    }
    pub fn open_inner_list(self) -> RefInnerListSerializer<'a, Self> {
        if !self.buffer.is_empty() {
            self.buffer.push_str(", ");
        }
        self.buffer.push('(');
        RefInnerListSerializer::<RefListSerializer> {
            buffer: self.buffer,
            caller_type: PhantomData,
        }
    }
}

/// Serializes `Dictionary` field value components incrementally.
/// ```
/// use sfv::{Decimal, FromPrimitive, RefBareItem, RefDictSerializer};
///
/// # fn main() -> Result<(), &'static str> {
/// let mut serialized_item = String::new();
///
/// RefDictSerializer::new(&mut serialized_item)
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
///    )?;
///
/// assert_eq!(
///    serialized_item,
///    "member1=11;foo, member2=(abc;abc_param=?0 def);bar=\"val\", member3=12.346"
/// );
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct RefDictSerializer<'a> {
    buffer: &'a mut String,
}

impl<'a> RefDictSerializer<'a> {
    pub fn new(buffer: &'a mut String) -> Self {
        RefDictSerializer { buffer }
    }

    pub fn bare_item_member(self, name: &str, value: impl AsRefBareItem) -> SFVResult<Self> {
        if !self.buffer.is_empty() {
            self.buffer.push_str(", ");
        }
        Serializer::serialize_key(name, self.buffer)?;
        let value = value.as_ref_bare_item();
        if value != RefBareItem::Boolean(true) {
            self.buffer.push('=');
            Serializer::serialize_bare_item(value, self.buffer)?;
        }
        Ok(self)
    }

    pub fn parameter(self, name: &str, value: impl AsRefBareItem) -> SFVResult<Self> {
        if self.buffer.is_empty() {
            return Err("parameters must be serialized after bare item or inner list");
        }
        Serializer::serialize_parameter(name, value, self.buffer)?;
        Ok(RefDictSerializer {
            buffer: self.buffer,
        })
    }

    pub fn open_inner_list(self, name: &str) -> SFVResult<RefInnerListSerializer<'a, Self>> {
        if !self.buffer.is_empty() {
            self.buffer.push_str(", ");
        }
        Serializer::serialize_key(name, self.buffer)?;
        self.buffer.push_str("=(");
        Ok(RefInnerListSerializer::<RefDictSerializer> {
            buffer: self.buffer,
            caller_type: PhantomData,
        })
    }
}

/// Used by `RefItemSerializer`, `RefListSerializer`, `RefDictSerializer` to serialize `InnerList`.
#[derive(Debug)]
pub struct RefInnerListSerializer<'a, T> {
    buffer: &'a mut String,
    caller_type: PhantomData<T>,
}

impl<'a, T: Container<'a>> RefInnerListSerializer<'a, T> {
    pub fn inner_list_bare_item(self, bare_item: impl AsRefBareItem) -> SFVResult<Self> {
        if !self.buffer.is_empty() & !self.buffer.ends_with('(') {
            self.buffer.push(' ');
        }
        Serializer::serialize_bare_item(bare_item, self.buffer)?;
        Ok(RefInnerListSerializer {
            buffer: self.buffer,
            caller_type: PhantomData,
        })
    }

    pub fn inner_list_parameter(self, name: &str, value: impl AsRefBareItem) -> SFVResult<Self> {
        if self.buffer.is_empty() {
            return Err("parameters must be serialized after bare item or inner list");
        }
        Serializer::serialize_parameter(name, value, self.buffer)?;
        Ok(RefInnerListSerializer {
            buffer: self.buffer,
            caller_type: PhantomData,
        })
    }

    pub fn close_inner_list(self) -> T {
        self.buffer.push(')');
        T::new(self.buffer)
    }
}

pub trait Container<'a> {
    fn new(buffer: &'a mut String) -> Self;
}

impl<'a> Container<'a> for RefListSerializer<'a> {
    fn new(buffer: &mut String) -> RefListSerializer {
        RefListSerializer { buffer }
    }
}

impl<'a> Container<'a> for RefDictSerializer<'a> {
    fn new(buffer: &mut String) -> RefDictSerializer {
        RefDictSerializer { buffer }
    }
}

#[cfg(test)]
mod alternative_serializer_tests {
    use super::*;
    use crate::{Decimal, FromPrimitive};

    #[test]
    fn test_fast_serialize_item() -> SFVResult<()> {
        let mut output = String::new();
        let ser = RefItemSerializer::new(&mut output);
        ser.bare_item(RefBareItem::Token("hello"))?
            .parameter("abc", true)?;
        assert_eq!("hello;abc", output);
        Ok(())
    }

    #[test]
    fn test_fast_serialize_list() -> SFVResult<()> {
        let mut output = String::new();
        let ser = RefListSerializer::new(&mut output);
        ser.bare_item(RefBareItem::Token("hello"))?
            .parameter("key1", true)?
            .parameter("key2", false)?
            .open_inner_list()
            .inner_list_bare_item(RefBareItem::String("some_string"))?
            .inner_list_bare_item(12)?
            .inner_list_parameter("inner-member-key", true)?
            .close_inner_list()
            .parameter("inner-list-param", RefBareItem::Token("*"))?;
        assert_eq!(
            "hello;key1;key2=?0, (\"some_string\" 12;inner-member-key);inner-list-param=*",
            output
        );
        Ok(())
    }

    #[test]
    fn test_fast_serialize_dict() -> SFVResult<()> {
        let mut output = String::new();
        let ser = RefDictSerializer::new(&mut output);
        ser.bare_item_member("member1", RefBareItem::Token("hello"))?
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
            .bare_item_member("key8", true)?;
        assert_eq!(
            "member1=hello;key1;key2=?0, member2;key3=45.459;key4=\"str\", key5=(45 0), key6=\"foo\", key7=(:c29tZV9zdHJpbmc=: :b3RoZXJfc3RyaW5n:);lparam=10, key8",
            output
        );
        Ok(())
    }
}
