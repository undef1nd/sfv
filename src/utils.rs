use std::iter::Peekable;
use std::str::Chars;

pub fn is_tchar(c: char) -> bool {
    // See tchar values list in https://tools.ietf.org/html/rfc7230#section-3.2.6
    let tchars = "!#$%&'*+-.^_`|~";
    tchars.contains(c) || c.is_ascii_alphanumeric()
}

pub fn consume_ows_chars(input_chars: &mut Peekable<Chars>) {
    while let Some(c) = input_chars.peek() {
        if c == &' ' || c == &'\t' {
            input_chars.next();
        } else {
            break;
        }
    }
}

pub fn consume_sp_chars(input_chars: &mut Peekable<Chars>) {
    while let Some(c) = input_chars.peek() {
        if c == &' ' {
            input_chars.next();
        } else {
            break;
        }
    }
}
