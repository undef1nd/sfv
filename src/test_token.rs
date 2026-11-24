use crate::{Token, TokenRef, error::StringError};

#[test]
#[should_panic = "cannot be empty"]
fn test_constant_empty() {
    let _ = TokenRef::constant("");
}

#[test]
#[should_panic = "invalid character"]
fn test_constant_invalid_start_char() {
    let _ = TokenRef::constant("#some");
}

#[test]
#[should_panic = "invalid character"]
fn test_constant_invalid_inner_char() {
    let _ = TokenRef::constant("s ");
}

#[test]
fn test_conversions() {
    assert_eq!(TokenRef::from_str(""), Err(StringError::Empty.into()));
    assert_eq!(
        TokenRef::from_str("#some"),
        Err(StringError::InvalidCharacter(0).into()),
    );
    assert_eq!(
        TokenRef::from_str("s "),
        Err(StringError::InvalidCharacter(1).into()),
    );
    assert_eq!(
        TokenRef::from_str("abc:de\t"),
        Err(StringError::InvalidCharacter(6).into()),
    );

    assert_eq!(TokenRef::from_bytes(b"abc:de").unwrap().as_str(), "abc:de");
    assert_eq!(
        TokenRef::from_bytes(b"abc\xff"),
        Err(StringError::InvalidCharacter(3).into()),
    );

    assert_eq!(
        Token::from_string("abc:de".to_owned()).unwrap().as_str(),
        "abc:de",
    );
    assert_eq!(
        Token::from_string("s ".to_owned()),
        Err((StringError::InvalidCharacter(1).into(), "s ".to_owned())),
    );
}
