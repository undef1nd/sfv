use crate::{
    key_ref, string_ref, token_ref, Decimal, DictSerializer, ItemSerializer, ListSerializer,
    SFVResult,
};

use std::borrow::BorrowMut;

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
            r#"hello;key1;key2=?0, ("some_string" 12;inner-member-key);inner-list-param=*"#,
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
            r#"member1=hello;key1;key2=?0, member2;key3=45.459;key4="str", key5=(45 0), key6="foo", key7=(:c29tZV9zdHJpbmc=: :b3RoZXJfc3RyaW5n:);lparam=10, key8"#,
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
