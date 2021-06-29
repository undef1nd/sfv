use crate::utils;
use crate::{
    BareItem, Decimal, Dictionary, FromStr, InnerList, Item, List, ListEntry, Num, Parameters,
    SFVResult,
};
use std::iter::Peekable;
use std::str::{from_utf8, CharIndices};

/// `BareItem` type is used to construct `Items` or `Parameters` values.
#[derive(Debug, PartialEq, Clone)]
pub enum BareItemRef<'a> {
    /// Decimal number
    // sf-decimal  = ["-"] 1*12DIGIT "." 1*3DIGIT
    Decimal(Decimal),
    /// Integer number
    // sf-integer = ["-"] 1*15DIGIT
    Integer(i64),
    // sf-string = DQUOTE *chr DQUOTE
    // chr       = unescaped / escaped
    // unescaped = %x20-21 / %x23-5B / %x5D-7E
    // escaped   = "\" ( DQUOTE / "\" )
    String(&'a str),
    // ":" *(base64) ":"
    // base64    = ALPHA / DIGIT / "+" / "/" / "="
    ByteSeq(&'a str),
    // sf-boolean = "?" boolean
    // boolean    = "0" / "1"
    Boolean(bool),
    // sf-token = ( ALPHA / "*" ) *( tchar / ":" / "/" )
    Token(&'a str),
}

#[derive(Debug, PartialEq, Clone)]
struct ParamRef<'a> {
    key: &'a str,
    value: &'a BareItemRef<'a>,
}

#[derive(Debug, PartialEq, Clone)]
struct ParamRefIterator<'a> {
    content: &'a str,
    index: u32,
}

impl<'a> Iterator for ParamRefIterator<'a> {
    type Item = ParamRef<'a>;
    fn next(&mut self) -> Option<ParamRef<'a>> {
        None
    }
}

#[test]
fn testRefIterator() {
    assert_eq!(RefParser::parse_item(b"?1;foo=bar;baz=fooz").unwrap().bare_item, BareItemRef::Boolean(true));
    assert_eq!(RefParser::parse_item(b"?0;foo=bar;baz=fooz").unwrap().bare_item, BareItemRef::Boolean(false));
    assert_eq!(RefParser::parse_item(b"123;foo=bar;baz=fooz").unwrap().bare_item, BareItemRef::Integer(123));
    assert_eq!(RefParser::parse_item(b"123.123;foo=bar;baz=fooz").unwrap().bare_item, BareItemRef::Decimal(Decimal::from_str("123.123").unwrap().into()));

    println!("{:?}",RefParser::parse_item(b":YmFzZV82NCBlbmNvZGluZyB0ZXN0:;foo=bar;baz=fooz").unwrap().bare_item);
    assert_eq!(
        RefParser::parse_item(b":YmFzZV82NCBlbmNvZGluZyB0ZXN0:;foo=bar;baz=fooz").unwrap().bare_item,
        BareItemRef::ByteSeq("YmFzZV82NCBlbmNvZGluZyB0ZXN0")
    );
    assert_eq!(
        RefParser::parse_item(b"\"abc\";foo=bar;baz=fooz").unwrap().bare_item,
        BareItemRef::String("abc")
    );

    assert_eq!(
        RefParser::parse_item(b"token;foo=bar;baz=fooz").unwrap().bare_item,
        BareItemRef::Token("token")
    );
}

// use crate::utils;
// use crate::{
//     BareItem, Decimal, Dictionary, FromStr, InnerList, Item, List, ListEntry, Num, Parameters,
//     SFVResult,
// };
// use std::iter::Peekable;
// use std::str::{from_utf8, Chars};

/// Implements parsing logic for each structured field value type.
// pub trait RefParseValue<'a> {
//     /// This method should not be used for parsing input into structured field value.
//     /// Use `RefParser::parse_item`, `RefParser::parse_list` or `RefParsers::parse_dictionary` for that.
//     fn parse(input: &'a mut SfvIterator) -> SFVResult<Self>
//     where
//         Self: Sized;
// }

pub(crate) struct SfvIterator<'a> {
    content: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

/// If structured field value of List or Dictionary type is split into multiple lines,
/// allows to parse more lines and merge them into already existing structure field value.
// pub trait RefParseMore {
//     /// If structured field value is split across lines,
//     /// parses and merges next line into a single structured field value.
//     /// # Examples
//     /// ```
//     /// # use sfv::{RefParser, SerializeValue, RefParseMore};
//     ///
//     /// let mut list_field = RefParser::parse_list("11, (12 13)".as_bytes()).unwrap();
//     /// list_field.parse_more("\"foo\",        \"bar\"".as_bytes()).unwrap();
//     ///
//     /// assert_eq!(list_field.serialize_value().unwrap(), "11, (12 13), \"foo\", \"bar\"");
//     fn parse_more(&mut self, input_bytes: &[u8]) -> SFVResult<()>
//     where
//         Self: Sized;
// }

#[derive(Debug, PartialEq, Clone)]
pub struct ItemRef<'a> {
    bare_item: BareItemRef<'a>,
    params: ParamRefIterator<'a>,
}

// impl<'a> RefParseValue for ItemRef<'a> {
//     fn parse(input: &'a mut SfvIterator) -> SFVResult<ItemRef<'a>> {
//         // https://httpwg.org/specs/rfc8941.html#parse-item
//         let bare_item = RefParser::parse_bare_item(input)?;
//         let params = ParamRefIterator { content: "todo", index: 0}; // RefParser::parse_parameters(input)?;

//         Ok(ItemRef { bare_item, params })
//     }
// }

// impl RefParseValue for List {
//     fn parse(input: &mut SfvIterator) -> SFVResult<List> {
//         // https://httpwg.org/specs/rfc8941.html#parse-list
//         // List represents an array of (item_or_inner_list, parameters)

//         let mut members = vec![];

//         while input.chars.peek().is_some() {
//             members.push(RefParser::parse_list_entry(input)?);

//             utils::consume_ows_chars(input);

//             if input.chars.peek().is_none() {
//                 return Ok(members);
//             }

//             if let Some(c) = input.next() {
//                 if c != ',' {
//                     return Err("parse_list: trailing characters after list member");
//                 }
//             }

//             utils::consume_ows_chars(input);

//             if input.chars.peek().is_none() {
//                 return Err("parse_list: trailing comma");
//             }
//         }

//         Ok(members)
//     }
// }

// impl RefParseValue for Dictionary {
//     fn parse(input: &mut SfvIterator) -> SFVResult<Dictionary> {
//         let mut dict = Dictionary::new();

//         while input.chars.peek().is_some() {
//             let this_key = RefParser::parse_key(input)?;

//             if let Some('=') = input.chars.peek() {
//                 input.next();
//                 let member = RefParser::parse_list_entry(input)?;
//                 dict.insert(this_key, member);
//             } else {
//                 let value = true;
//                 let params = RefParser::parse_parameters(input)?;
//                 let member = Item {
//                     bare_item: BareItem::Boolean(value),
//                     params,
//                 };
//                 dict.insert(this_key, member.into());
//             }

//             utils::consume_ows_chars(input);

//             if input.chars.peek().is_none() {
//                 return Ok(dict);
//             }

//             if let Some(c) = input.next() {
//                 if c != ',' {
//                     return Err("parse_dict: trailing characters after dictionary member");
//                 }
//             }

//             utils::consume_ows_chars(input);

//             if input.chars.peek().is_none() {
//                 return Err("parse_dict: trailing comma");
//             }
//         }
//         Ok(dict)
//     }
// }

// impl RefParseMore for List {
//     fn parse_more(&mut self, input_bytes: &[u8]) -> SFVResult<()> {
//         let parsed_list = RefParser::parse_list(input_bytes)?;
//         self.extend(parsed_list);
//         Ok(())
//     }
// }

// impl RefParseMore for Dictionary {
//     fn parse_more(&mut self, input_bytes: &[u8]) -> SFVResult<()> {
//         let parsed_dict = RefParser::parse_dictionary(input_bytes)?;
//         self.extend(parsed_dict);
//         Ok(())
//     }
// }

/// Exposes methods for parsing input into structured field value.
pub struct RefParser;

impl RefParser {
    /// Parses input into structured field value of Dictionary type
    // pub fn parse_dictionary(input_bytes: &[u8]) -> SFVResult<Dictionary> {
    //     Self::parse::<Dictionary>(input_bytes)
    // }

    // /// Parses input into structured field value of List type
    // pub fn parse_list(input_bytes: &[u8]) -> SFVResult<List> {
    //     Self::parse::<List>(input_bytes)
    // }

    /// Parses input into structured field value of Item type
    pub fn parse_item<'a>(input_bytes: &'a [u8]) -> SFVResult<ItemRef<'a>> {
        // Self::parse::<ItemRef>(input_bytes)

        if !input_bytes.is_ascii() {
            return Err("parse: non-ascii characters in input");
        }

        let mut input = from_utf8(input_bytes)
            .map_err(|_| "parse: conversion from bytes to str failed")?;
        let mut iter = SfvIterator{content: &input, chars: input.char_indices().peekable()};
        utils::consume_sp_chars_index(&mut iter.chars);

        let bare_item = RefParser::parse_bare_item(&mut iter)?;

        // TODO: Parse all params once. Then do it again and return first one.

        let params = ParamRefIterator { content: input, index: 0}; // RefParser::parse_parameters(input)?;

        let result = Ok(ItemRef { bare_item, params });

        utils::consume_sp_chars_index(&mut iter.chars);

        // if iter.chars.next().is_some() {
        //     return Err("parse: trailing characters after parsed value");
        // };

        result
    }

    // Generic parse method for checking input before parsing
    // and handling trailing text error
    // fn parse<'a, T: RefParseValue>(input_bytes: &'a [u8]) -> SFVResult<T> {
    //     // https://httpwg.org/specs/rfc8941.html#text-parse
    //     if !input_bytes.is_ascii() {
    //         return Err("parse: non-ascii characters in input");
    //     }

    //     let mut input = from_utf8(input_bytes)
    //         .map_err(|_| "parse: conversion from bytes to str failed")?;
    //     let mut mypeek = SfvIterator{content: &input, iterator: &mut input.char_indices().peekable()};
    //     utils::consume_sp_chars_index(&mut mypeek.chars);

    //     let output = T::parse(&mut mypeek)?;

    //     utils::consume_sp_chars_index(&mut mypeek.chars);

    //     if mypeek.chars.next().is_some() {
    //         return Err("parse: trailing characters after parsed value");
    //     };
    //     Ok(output)
    // }

    // fn parse_list_entry(input: &mut SfvIterator) -> SFVResult<ListEntry> {
    //     // https://httpwg.org/specs/rfc8941.html#parse-item-or-list
    //     // ListEntry represents a tuple (item_or_inner_list, parameters)

    //     match input.chars.peek() {
    //         Some((index,'(')) => {
    //             let parsed = Self::parse_inner_list(input)?;
    //             Ok(ListEntry::InnerList(parsed))
    //         }
    //         _ => {
    //             let parsed = Item::parse(input)?;
    //             Ok(ListEntry::Item(parsed))
    //         }
    //     }
    // }

    // pub(crate) fn parse_inner_list(input: &mut SfvIterator) -> SFVResult<InnerList> {
    //     // https://httpwg.org/specs/rfc8941.html#parse-innerlist

    //     if Some('(') != input.next() {
    //         return Err("parse_inner_list: input does not start with '('");
    //     }

    //     let mut inner_list = Vec::new();
    //     while input.chars.peek().is_some() {
    //         utils::consume_sp_chars_index(input);

    //         if Some(&')') == input.chars.peek() {
    //             input.next();
    //             let params = Self::parse_parameters(input)?;
    //             return Ok(InnerList {
    //                 items: inner_list,
    //                 params,
    //             });
    //         }

    //         let parsed_item = Item::parse(input)?;
    //         inner_list.push(parsed_item);

    //         if let Some(c) = input.chars.peek() {
    //             if c != &' ' && c != &')' {
    //                 return Err("parse_inner_list: bad delimitation");
    //             }
    //         }
    //     }

    //     Err("parse_inner_list: the end of the inner list was not found")
    // }

    pub(crate) fn parse_bare_item<'a>(mut input: &mut SfvIterator<'a>) -> SFVResult<BareItemRef<'a>> {
        // https://httpwg.org/specs/rfc8941.html#parse-bare-item
        if input.chars.peek().is_none() {
            return Err("parse_bare_item: empty item");
        }

        match input.chars.peek() {
            Some((_, '?')) => Self::parse_bool(&mut input),
            Some((_, '"')) => Self::parse_string(&mut input),
            Some((_, ':')) => Self::parse_byte_sequence(
                &mut input,
            ),
            Some((_,c)) if *c == '*' || c.is_ascii_alphabetic() => {
                Self::parse_token(&mut input)
            }
            Some((_,c)) if c == &'-' || c.is_ascii_digit() =>  Self::parse_number(&mut input),
            _ => Err("parse_bare_item: item type can't be identified"),
        }
    }

    pub(crate) fn parse_bool<'a>(input: &mut SfvIterator<'a>) -> SFVResult<BareItemRef<'a>> {
        // https://httpwg.org/specs/rfc8941.html#parse-boolean

        match input.chars.next() {
            Some((_, '?')) => {},
            _ => { return Err("parse_bool: first character is not '?'"); }
        }

        match input.chars.next() {
            Some((_,'0')) => Ok(BareItemRef::Boolean(false)),
            Some((_,'1')) => Ok(BareItemRef::Boolean(true)),
            _ => Err("parse_bool: invalid variant"),
        }
    }

    pub(crate) fn parse_string<'a>(input: &mut SfvIterator<'a>) -> SFVResult<BareItemRef<'a>> {
        // https://httpwg.org/specs/rfc8941.html#parse-string

        let start = match input.chars.next() {
            Some((index,'\"')) => index + 1,
            _ => {return Err("parse_string: first character is not '\"'");}
        };

        while let Some((index, curr_char)) = input.chars.next() {
            match curr_char {
                '\"' => { return Ok(BareItemRef::String(&input.content[start..index]));},
                '\x7f' | '\x00'..='\x1f' => return Err("parse_string: not a visible character"),
                '\\' => match input.chars.next() {
                    Some((_,c)) if c == '\\' || c == '\"' => {
                    }
                    None => return Err("parse_string: last input character is '\\'"),
                    _ => return Err("parse_string: disallowed character after '\\'"),
                },
                _ => {},
            }
        }
        Err("parse_string: no closing '\"'")
    }

    pub(crate) fn parse_token<'a>(input: &mut SfvIterator<'a>) -> SFVResult<BareItemRef<'a>> {
        // https://httpwg.org/specs/rfc8941.html#parse-token


        let start : usize = match input.chars.peek() {
            None => return Err("parse_token: empty input string"),
            Some((index, c)) if !c.is_ascii_alphabetic() && c != &'*' => return Err("parse_token: first character is not ALPHA or '*'"),
            Some((index, _)) => *index,
        };

        while let Some((index, curr_char)) = input.chars.peek() {
            if !utils::is_tchar(*curr_char) && curr_char != &':' && curr_char != &'/' {
                return Ok(BareItemRef::Token(&input.content[start..*index]));
            }

            match input.chars.next() {
                Some(c) => {},
                None => return Err("parse_token: end of the string"),
            }
        }
        Ok(BareItemRef::Token(&input.content[start..]))
    }

    pub(crate) fn parse_byte_sequence<'a>(input: &mut SfvIterator<'a>) -> SFVResult<BareItemRef<'a>> {
        // https://httpwg.org/specs/rfc8941.html#parse-binary

        let start = match input.chars.next() {
            Some((index, ':')) => index + 1,
            _ => return Err("parse_byte_seq: first char is not ':'")
        };

        let end = match input.chars.clone().find(|(_, c)| c == &':') {
            Some((index, ':')) => index,
            _ => return Err("parse_byte_seq: no closing ':'")
        };

        let substring = &input.content[start..end];

        // Check if it's valid base64
        if !substring.chars().all(utils::is_allowed_b64_content) {
            return Err("parse_byte_seq: invalid char in byte sequence");
        }

        match utils::base64()?.decode(substring.as_bytes()) {
            Err(_) => return Err("parse_byte_seq: decoding error"),
            _ => {}
        }

        Ok(BareItemRef::ByteSeq(substring))
    }

    pub(crate) fn parse_number<'a>(input: &mut SfvIterator<'a>) -> SFVResult<BareItemRef<'a>> {
        // https://httpwg.org/specs/rfc8941.html#parse-number

        let mut sign = 1;
        if let Some((_,'-')) = input.chars.peek() {
            sign = -1;
            input.chars.next();
        }

        match input.chars.peek() {
            Some((_,c)) if !c.is_ascii_digit() => {
                return Err("parse_number: input number does not start with a digit")
            }
            None => return Err("parse_number: input number lacks a digit"),
            _ => (),
        }

        // Get number from input as a string and identify whether it's a decimal or integer
        let (is_integer, input_number) = Self::extract_digits(input)?;

        // Parse input_number from string into integer
        if is_integer {
            let output_number = input_number
                .parse::<i64>()
                .map_err(|_err| "parse_number: parsing i64 failed")?
                * sign;

            let (min_int, max_int) = (-999_999_999_999_999_i64, 999_999_999_999_999_i64);
            if !(min_int <= output_number && output_number <= max_int) {
                return Err("parse_number: integer number is out of range");
            }

            return Ok(BareItemRef::Integer(output_number));
        }

        // Parse input_number from string into decimal
        let chars_after_dot = input_number
            .find('.')
            .map(|dot_pos| input_number.len() - dot_pos - 1);

        match chars_after_dot {
            Some(0) => Err("parse_number: decimal ends with '.'"),
            Some(1..=3) => {
                let mut output_number = Decimal::from_str(&input_number)
                    .map_err(|_err| "parse_number: parsing f64 failed")?;

                if sign == -1 {
                    output_number.set_sign_negative(true)
                }

                Ok(BareItemRef::Decimal(output_number))
            }
            _ => Err("parse_number: invalid decimal fraction length"),
        }
    }

    fn extract_digits(input: &mut SfvIterator) -> SFVResult<(bool, String)> {
        let mut is_integer = true;
        let mut input_number = String::from("");
        while let Some((_,curr_char)) = input.chars.peek() {
            if curr_char.is_ascii_digit() {
                input_number.push(*curr_char);
                input.chars.next();
            } else if curr_char == &'.' && is_integer {
                if input_number.len() > 12 {
                    return Err(
                        "parse_number: decimal too long, illegal position for decimal point",
                    );
                }
                input_number.push(*curr_char);
                is_integer = false;
                input.chars.next();
            } else {
                break;
            }

            if is_integer && input_number.len() > 15 {
                return Err("parse_number: integer too long, length > 15");
            }

            if !is_integer && input_number.len() > 16 {
                return Err("parse_number: decimal too long, length > 16");
            }
        }
        Ok((is_integer, input_number))
    }

    // pub(crate) fn parse_parameters(input: &mut SfvIterator) -> SFVResult<Parameters> {
    //     // https://httpwg.org/specs/rfc8941.html#parse-param

    //     // let mut params = Parameters::new();

    //     while let Some((_,curr_char)) = input.chars.peek() {
    //         if curr_char == &';' {
    //             input.chars.next();
    //         } else {
    //             break;
    //         }

    //         utils::consume_sp_chars_index(input.chars);

    //         let param_name = Self::parse_key(input)?;
    //         let param_value = match input.chars.peek() {
    //             Some((_,'=')) => {
    //                 input.chars.next();
    //                 Self::parse_bare_item(input)?
    //             }
    //             _ => BareItemRef::Boolean(true),
    //         };
    //         // params.insert(param_name, param_value);
    //     }

    //     // If parameters already contains a name param_name (comparing character-for-character), overwrite its value.
    //     // Note that when duplicate Parameter keys are encountered, this has the effect of ignoring all but the last instance.
    //     Ok(params)
    // }

    pub(crate) fn parse_key(input: &mut SfvIterator) -> SFVResult<String> {
        match input.chars.peek() {
            Some((_,c)) if c == &'*' || c.is_ascii_lowercase() => (),
            _ => return Err("parse_key: first character is not lcalpha or '*'"),
        }

        let mut output = String::new();
        while let Some((_,curr_char)) = input.chars.peek() {
            if !curr_char.is_ascii_lowercase()
                && !curr_char.is_ascii_digit()
                && !"_-*.".contains(*curr_char)
            {
                return Ok(output);
            }

            output.push(*curr_char);
            input.chars.next();
        }
        Ok(output)
    }
}
