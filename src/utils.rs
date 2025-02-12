use base64::engine;

pub(crate) const BASE64: engine::GeneralPurpose = engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    engine::GeneralPurposeConfig::new()
        .with_decode_allow_trailing_bits(true)
        .with_decode_padding_mode(engine::DecodePaddingMode::Indifferent)
        .with_encode_padding(true),
);

fn is_tchar(c: u8) -> bool {
    // See tchar values list in https://tools.ietf.org/html/rfc7230#section-3.2.6
    let tchars = b"!#$%&'*+-.^_`|~";
    tchars.contains(&c) || c.is_ascii_alphanumeric()
}

pub(crate) fn is_allowed_start_token_char(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'*'
}

pub(crate) fn is_allowed_inner_token_char(c: u8) -> bool {
    is_tchar(c) || c == b':' || c == b'/'
}
