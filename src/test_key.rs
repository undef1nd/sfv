use crate::{Key, KeyRef, error::StringError};

#[test]
#[should_panic = "cannot be empty"]
fn test_constant_empty() {
    let _ = KeyRef::constant("");
}

#[test]
#[should_panic = "invalid character"]
fn test_constant_invalid_start_char() {
    let _ = KeyRef::constant("_key");
}

#[test]
#[should_panic = "invalid character"]
fn test_constant_invalid_inner_char() {
    let _ = KeyRef::constant("aND");
}

#[test]
fn test_conversions() {
    assert_eq!(KeyRef::from_str(""), Err(StringError::Empty.into()));
    assert_eq!(
        KeyRef::from_str("aND"),
        Err(StringError::InvalidCharacter(1).into()),
    );
    assert_eq!(
        KeyRef::from_str("_key"),
        Err(StringError::InvalidCharacter(0).into()),
    );
    assert_eq!(
        KeyRef::from_str("7key"),
        Err(StringError::InvalidCharacter(0).into()),
    );

    assert_eq!(KeyRef::from_bytes(b"a-1").unwrap().as_str(), "a-1");
    assert_eq!(
        KeyRef::from_bytes(b"ab\xff"),
        Err(StringError::InvalidCharacter(2).into()),
    );

    assert_eq!(Key::try_from("a-1".to_owned()).unwrap().as_str(), "a-1");
    assert_eq!(
        Key::try_from("aND".to_owned()),
        Err(StringError::InvalidCharacter(1).into()),
    );

    assert_eq!(Key::from_string("a-1".to_owned()).unwrap().as_str(), "a-1");
    assert_eq!(
        Key::from_string("aND".to_owned()),
        Err((StringError::InvalidCharacter(1).into(), "aND".to_owned())),
    );
}
