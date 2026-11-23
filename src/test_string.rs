use crate::{String, StringRef, error::StringError};

#[test]
#[should_panic = "invalid character"]
fn test_constant_invalid_char() {
    let _ = StringRef::constant("text \x00");
}

#[test]
fn test_conversions() {
    assert_eq!(
        StringRef::from_str("text \x00"),
        Err(StringError::InvalidCharacter(5).into()),
    );
    assert_eq!(
        StringRef::from_str("text \x1f"),
        Err(StringError::InvalidCharacter(5).into()),
    );
    assert_eq!(
        StringRef::from_str("text \x7f"),
        Err(StringError::InvalidCharacter(5).into()),
    );
    assert_eq!(
        StringRef::from_str("рядок"),
        Err(StringError::InvalidCharacter(0).into()),
    );
    assert_eq!(
        StringRef::from_str("non-ascii text 🐹"),
        Err(StringError::InvalidCharacter(15).into()),
    );

    assert_eq!(StringRef::from_bytes(b"hello").unwrap().as_str(), "hello");
    assert_eq!(
        StringRef::from_bytes(b"hi\xff"),
        Err(StringError::InvalidCharacter(2).into()),
    );

    assert_eq!(
        String::from_string("hello".to_owned()).unwrap().as_str(),
        "hello",
    );
    assert_eq!(
        String::from_string("text \x7f".to_owned()),
        Err((
            StringError::InvalidCharacter(5).into(),
            "text \x7f".to_owned(),
        )),
    );
}
