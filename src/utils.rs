use base64::engine;
use std::iter::Peekable;

pub(crate) const BASE64: engine::GeneralPurpose = engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    engine::GeneralPurposeConfig::new()
        .with_decode_allow_trailing_bits(true)
        .with_decode_padding_mode(engine::DecodePaddingMode::Indifferent)
        .with_encode_padding(true),
);

pub(crate) fn is_tchar(c: char) -> bool {
    // See tchar values list in https://tools.ietf.org/html/rfc7230#section-3.2.6
    let tchars = "!#$%&'*+-.^_`|~";
    tchars.contains(c) || c.is_ascii_alphanumeric()
}

pub(crate) fn consume_ows_chars(input_chars: &mut Peekable<impl Iterator<Item = char>>) {
    while let Some(c) = input_chars.peek() {
        if c == &' ' || c == &'\t' {
            input_chars.next();
        } else {
            break;
        }
    }
}

pub(crate) fn consume_sp_chars(input_chars: &mut Peekable<impl Iterator<Item = char>>) {
    while let Some(c) = input_chars.peek() {
        if c == &' ' {
            input_chars.next();
        } else {
            break;
        }
    }
}
