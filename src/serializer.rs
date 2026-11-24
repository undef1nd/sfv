use std::fmt::Write as _;

use crate::{Date, Decimal, Integer, KeyRef, RefBareItem, StringRef, TokenRef, utils};

pub(crate) fn serialize_bare_item<'b>(value: impl Into<RefBareItem<'b>>, output: &mut String) {
    // https://httpwg.org/specs/rfc9651.html#ser-bare-item

    match value.into() {
        RefBareItem::Boolean(value) => serialize_bool(value, output),
        RefBareItem::String(value) => serialize_string(value, output),
        RefBareItem::ByteSequence(value) => serialize_byte_sequence(value, output),
        RefBareItem::Token(value) => serialize_token(value, output),
        RefBareItem::Integer(value) => serialize_integer(value, output),
        RefBareItem::Decimal(value) => serialize_decimal(value, output),
        RefBareItem::Date(value) => serialize_date(value, output),
        RefBareItem::DisplayString(value) => serialize_display_string(value, output),
    }
}

pub(crate) fn serialize_parameter<'b>(
    name: &KeyRef,
    value: impl Into<RefBareItem<'b>>,
    output: &mut String,
) {
    // https://httpwg.org/specs/rfc9651.html#ser-params
    output.push(';');
    serialize_key(name, output);

    let value = value.into();
    if value != RefBareItem::Boolean(true) {
        output.push('=');
        serialize_bare_item(value, output);
    }
}

pub(crate) fn serialize_key(input_key: &KeyRef, output: &mut String) {
    // https://httpwg.org/specs/rfc9651.html#ser-key

    output.push_str(input_key.as_str());
}

pub(crate) fn serialize_integer(value: Integer, output: &mut String) {
    //https://httpwg.org/specs/rfc9651.html#ser-integer

    // TODO: Use `std::fmt::NumBuffer` once MSRV is >= 1.98.0.
    write!(output, "{value}").unwrap();
}

pub(crate) fn serialize_decimal(value: Decimal, output: &mut String) {
    // https://httpwg.org/specs/rfc9651.html#ser-decimal

    // TODO: Use `std::fmt::NumBuffer` once MSRV is >= 1.98.0.
    write!(output, "{value}").unwrap();
}

pub(crate) fn serialize_string(value: &StringRef, output: &mut String) {
    // https://httpwg.org/specs/rfc9651.html#ser-string

    let s = value.as_str();
    output.reserve(s.len() + 2);
    output.push('"');

    let mut rest = s;
    while let Some(i) = rest.bytes().position(|b| b == b'\\' || b == b'"') {
        let (before, after) = rest.split_at(i);
        let (escaped, after) = after.split_at(1);
        output.push_str(before);
        output.push('\\');
        output.push_str(escaped);
        rest = after;
    }
    output.push_str(rest);
    output.push('"');
}

pub(crate) fn serialize_token(value: &TokenRef, output: &mut String) {
    // https://httpwg.org/specs/rfc9651.html#ser-token

    output.push_str(value.as_str());
}

pub(crate) fn serialize_byte_sequence(value: &[u8], output: &mut String) {
    // https://httpwg.org/specs/rfc9651.html#ser-binary

    output.push(':');
    base64::Engine::encode_string(&utils::BASE64, value, output);
    output.push(':');
}

pub(crate) fn serialize_bool(value: bool, output: &mut String) {
    // https://httpwg.org/specs/rfc9651.html#ser-boolean

    output.push_str(if value { "?1" } else { "?0" });
}

pub(crate) fn serialize_date(value: Date, output: &mut String) {
    // https://httpwg.org/specs/rfc9651.html#ser-date

    // TODO: Use `std::fmt::NumBuffer` once MSRV is >= 1.98.0.
    write!(output, "{value}").unwrap();
}

pub(crate) fn serialize_display_string(value: &str, output: &mut String) {
    // https://httpwg.org/specs/rfc9651.html#ser-display

    const HEX_DIGITS: [u8; 16] = *b"0123456789abcdef";

    output.reserve(value.len() + 3);
    output.push_str(r#"%""#);

    let mut span_start = 0;
    for (i, &c) in value.as_bytes().iter().enumerate() {
        if matches!(c, b'%' | b'"' | 0x00..=0x1f | 0x7f..=0xff) {
            if span_start < i {
                output.push_str(&value[span_start..i]);
            }
            output.push('%');
            output.push(HEX_DIGITS[usize::from(c >> 4)] as char);
            output.push(HEX_DIGITS[usize::from(c & 0x0f)] as char);
            span_start = i + 1;
        }
    }
    if span_start < value.len() {
        output.push_str(&value[span_start..]);
    }

    output.push('"');
}
