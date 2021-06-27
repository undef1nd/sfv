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
//     fn parse(input_chars: &'a mut MyPeek) -> SFVResult<Self>
//     where
//         Self: Sized;
// }

pub struct MyPeek<'a> {
    content: &'a str,
    iterator: Peekable<CharIndices<'a>>,
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
//     fn parse(input_chars: &'a mut MyPeek) -> SFVResult<ItemRef<'a>> {
//         // https://httpwg.org/specs/rfc8941.html#parse-item
//         let bare_item = RefParser::parse_bare_item(input_chars)?;
//         let params = ParamRefIterator { content: "todo", index: 0}; // RefParser::parse_parameters(input_chars)?;

//         Ok(ItemRef { bare_item, params })
//     }
// }

// impl RefParseValue for List {
//     fn parse(input_chars: &mut MyPeek) -> SFVResult<List> {
//         // https://httpwg.org/specs/rfc8941.html#parse-list
//         // List represents an array of (item_or_inner_list, parameters)

//         let mut members = vec![];

//         while input_chars.iterator.peek().is_some() {
//             members.push(RefParser::parse_list_entry(input_chars)?);

//             utils::consume_ows_chars(input_chars);

//             if input_chars.iterator.peek().is_none() {
//                 return Ok(members);
//             }

//             if let Some(c) = input_chars.next() {
//                 if c != ',' {
//                     return Err("parse_list: trailing characters after list member");
//                 }
//             }

//             utils::consume_ows_chars(input_chars);

//             if input_chars.iterator.peek().is_none() {
//                 return Err("parse_list: trailing comma");
//             }
//         }

//         Ok(members)
//     }
// }

// impl RefParseValue for Dictionary {
//     fn parse(input_chars: &mut MyPeek) -> SFVResult<Dictionary> {
//         let mut dict = Dictionary::new();

//         while input_chars.iterator.peek().is_some() {
//             let this_key = RefParser::parse_key(input_chars)?;

//             if let Some('=') = input_chars.iterator.peek() {
//                 input_chars.next();
//                 let member = RefParser::parse_list_entry(input_chars)?;
//                 dict.insert(this_key, member);
//             } else {
//                 let value = true;
//                 let params = RefParser::parse_parameters(input_chars)?;
//                 let member = Item {
//                     bare_item: BareItem::Boolean(value),
//                     params,
//                 };
//                 dict.insert(this_key, member.into());
//             }

//             utils::consume_ows_chars(input_chars);

//             if input_chars.iterator.peek().is_none() {
//                 return Ok(dict);
//             }

//             if let Some(c) = input_chars.next() {
//                 if c != ',' {
//                     return Err("parse_dict: trailing characters after dictionary member");
//                 }
//             }

//             utils::consume_ows_chars(input_chars);

//             if input_chars.iterator.peek().is_none() {
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

        let mut input_chars = from_utf8(input_bytes)
            .map_err(|_| "parse: conversion from bytes to str failed")?;
        let mut mypeek = MyPeek{content: &input_chars, iterator: input_chars.char_indices().peekable()};
        utils::consume_sp_chars_index(&mut mypeek.iterator);

        let bare_item = RefParser::parse_bare_item(&mut mypeek)?;

        // TODO: Parse all params once. Then do it again and return first one.

        let params = ParamRefIterator { content: input_chars, index: 0}; // RefParser::parse_parameters(input_chars)?;

        let result = Ok(ItemRef { bare_item, params });


        utils::consume_sp_chars_index(&mut mypeek.iterator);

        // if mypeek.iterator.next().is_some() {
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

    //     let mut input_chars = from_utf8(input_bytes)
    //         .map_err(|_| "parse: conversion from bytes to str failed")?;
    //     let mut mypeek = MyPeek{content: &input_chars, iterator: &mut input_chars.char_indices().peekable()};
    //     utils::consume_sp_chars_index(&mut mypeek.iterator);

    //     let output = T::parse(&mut mypeek)?;

    //     utils::consume_sp_chars_index(&mut mypeek.iterator);

    //     if mypeek.iterator.next().is_some() {
    //         return Err("parse: trailing characters after parsed value");
    //     };
    //     Ok(output)
    // }

    // fn parse_list_entry(input_chars: &mut MyPeek) -> SFVResult<ListEntry> {
    //     // https://httpwg.org/specs/rfc8941.html#parse-item-or-list
    //     // ListEntry represents a tuple (item_or_inner_list, parameters)

    //     match input_chars.iterator.peek() {
    //         Some((index,'(')) => {
    //             let parsed = Self::parse_inner_list(input_chars)?;
    //             Ok(ListEntry::InnerList(parsed))
    //         }
    //         _ => {
    //             let parsed = Item::parse(input_chars)?;
    //             Ok(ListEntry::Item(parsed))
    //         }
    //     }
    // }

    // pub(crate) fn parse_inner_list(input_chars: &mut MyPeek) -> SFVResult<InnerList> {
    //     // https://httpwg.org/specs/rfc8941.html#parse-innerlist

    //     if Some('(') != input_chars.next() {
    //         return Err("parse_inner_list: input does not start with '('");
    //     }

    //     let mut inner_list = Vec::new();
    //     while input_chars.iterator.peek().is_some() {
    //         utils::consume_sp_chars_index(input_chars);

    //         if Some(&')') == input_chars.iterator.peek() {
    //             input_chars.next();
    //             let params = Self::parse_parameters(input_chars)?;
    //             return Ok(InnerList {
    //                 items: inner_list,
    //                 params,
    //             });
    //         }

    //         let parsed_item = Item::parse(input_chars)?;
    //         inner_list.push(parsed_item);

    //         if let Some(c) = input_chars.iterator.peek() {
    //             if c != &' ' && c != &')' {
    //                 return Err("parse_inner_list: bad delimitation");
    //             }
    //         }
    //     }

    //     Err("parse_inner_list: the end of the inner list was not found")
    // }

    pub(crate) fn parse_bare_item<'a>(mut input_chars: &mut MyPeek<'a>) -> SFVResult<BareItemRef<'a>> {
        // https://httpwg.org/specs/rfc8941.html#parse-bare-item
        if input_chars.iterator.peek().is_none() {
            return Err("parse_bare_item: empty item");
        }

        match input_chars.iterator.peek() {
            Some((_, '?')) => Ok(BareItemRef::Boolean(Self::parse_bool(&mut input_chars)?)),
            Some((_, '"')) => Self::parse_string(&mut input_chars),
            Some((_, ':')) => Self::parse_byte_sequence(
                &mut input_chars,
            ),
            Some((_,c)) if *c == '*' || c.is_ascii_alphabetic() => {
                Self::parse_token(&mut input_chars)
            }
            Some((_,c)) if c == &'-' || c.is_ascii_digit() => {
                match Self::parse_number(&mut input_chars)? {
                    Num::Decimal(val) => Ok(BareItemRef::Decimal(val)),
                    Num::Integer(val) => Ok(BareItemRef::Integer(val)),
                }
            }
            _ => Err("parse_bare_item: item type can't be identified"),
        }
    }

    pub(crate) fn parse_bool(input_chars: &mut MyPeek) -> SFVResult<bool> {
        // https://httpwg.org/specs/rfc8941.html#parse-boolean

        match input_chars.iterator.next() {
            Some((_, '?')) => {},
            _ => { return Err("parse_bool: first character is not '?'"); }
        }

        match input_chars.iterator.next() {
            Some((_,'0')) => Ok(false),
            Some((_,'1')) => Ok(true),
            _ => Err("parse_bool: invalid variant"),
        }
    }

    pub(crate) fn parse_string<'a>(input_chars: &mut MyPeek<'a>) -> SFVResult<BareItemRef<'a>> {
        // https://httpwg.org/specs/rfc8941.html#parse-string

        let start = match input_chars.iterator.next() {
            Some((index,'\"')) => index + 1,
            _ => {return Err("parse_string: first character is not '\"'");}
        };

        while let Some((index, curr_char)) = input_chars.iterator.next() {
            match curr_char {
                '\"' => { return Ok(BareItemRef::String(&input_chars.content[start..index]));},
                '\x7f' | '\x00'..='\x1f' => return Err("parse_string: not a visible character"),
                '\\' => match input_chars.iterator.next() {
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

    pub(crate) fn parse_token<'a>(input_chars: &mut MyPeek<'a>) -> SFVResult<BareItemRef<'a>> {
        // https://httpwg.org/specs/rfc8941.html#parse-token


        let start : usize = match input_chars.iterator.peek() {
            None => return Err("parse_token: empty input string"),
            Some((index, c)) if !c.is_ascii_alphabetic() && c != &'*' => return Err("parse_token: first character is not ALPHA or '*'"),
            Some((index, _)) => *index,
        };

        while let Some((index, curr_char)) = input_chars.iterator.peek() {
            if !utils::is_tchar(*curr_char) && curr_char != &':' && curr_char != &'/' {
                return Ok(BareItemRef::Token(&input_chars.content[start..*index]));
            }

            match input_chars.iterator.next() {
                Some(c) => {},
                None => return Err("parse_token: end of the string"),
            }
        }
        Ok(BareItemRef::Token(&input_chars.content[start..]))
    }

    pub(crate) fn parse_byte_sequence<'a>(input_chars: &mut MyPeek<'a>) -> SFVResult<BareItemRef<'a>> {
        // https://httpwg.org/specs/rfc8941.html#parse-binary

        let start = match input_chars.iterator.next() {
            Some((index, ':')) => index + 1,
            _ => return Err("parse_byte_seq: first char is not ':'")
        };

        let end = match input_chars.iterator.clone().find(|(_, c)| c == &':') {
            Some((index, ':')) => index,
            _ => return Err("parse_byte_seq: no closing ':'")
        };

        Ok(BareItemRef::ByteSeq(&input_chars.content[start..end]))
        // TODO: check if it's valid base64
        // let b64_content = input_chars.iterator.take_while(|(_,c)| c != &':').collect::<String>();
        // if !b64_content.chars().all(utils::is_allowed_b64_content) {
        //     return Err("parse_byte_seq: invalid char in byte sequence");
        // }
        // match utils::base64()?.decode(b64_content.as_bytes()) {
        //     Ok(content) => Ok(content),
        //     Err(_) => Err("parse_byte_seq: decoding error"),
        // }
    }

    pub(crate) fn parse_number(input_chars: &mut MyPeek) -> SFVResult<Num> {
        // https://httpwg.org/specs/rfc8941.html#parse-number

        let mut sign = 1;
        if let Some((_,'-')) = input_chars.iterator.peek() {
            sign = -1;
            input_chars.iterator.next();
        }

        match input_chars.iterator.peek() {
            Some((_,c)) if !c.is_ascii_digit() => {
                return Err("parse_number: input number does not start with a digit")
            }
            None => return Err("parse_number: input number lacks a digit"),
            _ => (),
        }

        // Get number from input as a string and identify whether it's a decimal or integer
        let (is_integer, input_number) = Self::extract_digits(input_chars)?;

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

            return Ok(Num::Integer(output_number));
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

                Ok(Num::Decimal(output_number))
            }
            _ => Err("parse_number: invalid decimal fraction length"),
        }
    }

    fn extract_digits(input_chars: &mut MyPeek) -> SFVResult<(bool, String)> {
        let mut is_integer = true;
        let mut input_number = String::from("");
        while let Some((_,curr_char)) = input_chars.iterator.peek() {
            if curr_char.is_ascii_digit() {
                input_number.push(*curr_char);
                input_chars.iterator.next();
            } else if curr_char == &'.' && is_integer {
                if input_number.len() > 12 {
                    return Err(
                        "parse_number: decimal too long, illegal position for decimal point",
                    );
                }
                input_number.push(*curr_char);
                is_integer = false;
                input_chars.iterator.next();
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

    // pub(crate) fn parse_parameters(input_chars: &mut MyPeek) -> SFVResult<Parameters> {
    //     // https://httpwg.org/specs/rfc8941.html#parse-param

    //     // let mut params = Parameters::new();

    //     while let Some((_,curr_char)) = input_chars.iterator.peek() {
    //         if curr_char == &';' {
    //             input_chars.iterator.next();
    //         } else {
    //             break;
    //         }

    //         utils::consume_sp_chars_index(input_chars.iterator);

    //         let param_name = Self::parse_key(input_chars)?;
    //         let param_value = match input_chars.iterator.peek() {
    //             Some((_,'=')) => {
    //                 input_chars.iterator.next();
    //                 Self::parse_bare_item(input_chars)?
    //             }
    //             _ => BareItemRef::Boolean(true),
    //         };
    //         // params.insert(param_name, param_value);
    //     }

    //     // If parameters already contains a name param_name (comparing character-for-character), overwrite its value.
    //     // Note that when duplicate Parameter keys are encountered, this has the effect of ignoring all but the last instance.
    //     Ok(params)
    // }

    pub(crate) fn parse_key(input_chars: &mut MyPeek) -> SFVResult<String> {
        match input_chars.iterator.peek() {
            Some((_,c)) if c == &'*' || c.is_ascii_lowercase() => (),
            _ => return Err("parse_key: first character is not lcalpha or '*'"),
        }

        let mut output = String::new();
        while let Some((_,curr_char)) = input_chars.iterator.peek() {
            if !curr_char.is_ascii_lowercase()
                && !curr_char.is_ascii_digit()
                && !"_-*.".contains(*curr_char)
            {
                return Ok(output);
            }

            output.push(*curr_char);
            input_chars.iterator.next();
        }
        Ok(output)
    }
}
