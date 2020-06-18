use crate::parser::*;
use crate::utils;
use data_encoding::BASE64;
use rust_decimal::prelude::Zero;
use rust_decimal::Decimal;

type SerializerResult<T> = Result<T, &'static str>;

pub struct Serializer;

impl Serializer {
    pub fn serialize(header: &Header) -> SerializerResult<String> {
        let mut output = String::new();
        match header {
            Header::Item(value) => Self::serialize_item(value, &mut output)?,
            Header::List(value) => Self::serialize_list(value, &mut output)?,
            Header::Dictionary(value) => Self::serialize_dictionary(value, &mut output)?,
        };
        Ok(output)
    }

    fn serialize_dictionary(input_dict: &Dictionary, output: &mut String) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-dictionary

        for (idx, (member_name, member_value)) in input_dict.iter().enumerate() {
            Self::serialize_key(member_name, output)?;

            match member_value {
                ListEntry::Item(ref item) => {
                    // If dict member is boolean true, no need to serialize it: only its params must be serialized
                    // Otherwise serialize entire item with its params
                    if item.0 == BareItem::Boolean(true) {
                        Self::serialize_parameters(&item.1, output)?;
                    } else {
                        output.push('=');
                        Self::serialize_item(&item, output)?;
                    }
                }
                ListEntry::InnerList(inner_list) => {
                    output.push('=');
                    Self::serialize_inner_list(&inner_list, output)?;
                }
            }

            // If more items remain in input_dictionary:
            //      Append “,” to output.
            //      Append a single SP to output.
            if idx < input_dict.len() - 1 {
                output.push_str(", ");
            }
        }
        Ok(())
    }

    fn serialize_list(input_list: &List, output: &mut String) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-list

        for (idx, member) in input_list.iter().enumerate() {
            match member {
                ListEntry::Item(item) => {
                    Self::serialize_item(item, output)?;
                }
                ListEntry::InnerList(inner_list) => {
                    Self::serialize_inner_list(inner_list, output)?;
                }
            };

            // If more items remain in input_list:
            //      Append “,” to output.
            //      Append a single SP to output.
            if idx < input_list.len() - 1 {
                output.push_str(", ");
            }
        }
        Ok(())
    }

    fn serialize_inner_list(
        input_inner_list: &InnerList,
        output: &mut String,
    ) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-innerlist

        let items = &input_inner_list.0;
        let inner_list_parameters = &input_inner_list.1;

        output.push('(');
        for (idx, item) in items.iter().enumerate() {
            Self::serialize_item(item, output)?;

            // If more values remain in inner_list, append a single SP to output
            if idx < items.len() - 1 {
                output.push_str(" ");
            }
        }
        output.push(')');
        Self::serialize_parameters(inner_list_parameters, output)?;
        Ok(())
    }

    fn serialize_item(input_item: &Item, output: &mut String) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-item

        Self::serialize_bare_item(&input_item.0, output)?;
        Self::serialize_parameters(&input_item.1, output)?;
        Ok(())
    }

    fn serialize_bare_item(
        input_bare_item: &BareItem,
        output: &mut String,
    ) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-bare-item

        match input_bare_item {
            BareItem::Boolean(value) => Self::serialize_bool(*value, output)?,
            BareItem::String(value) => Self::serialize_string(value, output)?,
            BareItem::ByteSeq(value) => Self::serialize_byte_sequence(value, output)?,
            BareItem::Token(value) => Self::serialize_token(value, output)?,
            BareItem::Number(Num::Integer(value)) => Self::serialize_integer(*value, output)?,
            BareItem::Number(Num::Decimal(value)) => Self::serialize_decimal(*value, output)?,
        };
        Ok(())
    }

    fn serialize_parameters(
        input_params: &Parameters,
        output: &mut String,
    ) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-params

        for (param_name, param_value) in input_params.iter() {
            output.push(';');
            Self::serialize_key(param_name, output)?;

            if param_value != &BareItem::Boolean(true) {
                output.push('=');
                Self::serialize_bare_item(param_value, output)?;
            }
        }
        Ok(())
    }

    fn serialize_key(input_key: &str, output: &mut String) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-key

        let disallowed_chars =
            |c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || "_-*.".contains(c));

        if input_key.chars().any(disallowed_chars) {
            return Err("serialize_key: disallowed character in input");
        }

        if let Some(char) = input_key.chars().next() {
            if !(char.is_ascii_lowercase() || char == '*') {
                return Err("serialize_key: first character is not lcalpha or '*'");
            }
        }
        output.push_str(input_key);
        Ok(())
    }

    fn serialize_integer(value: i64, output: &mut String) -> SerializerResult<()> {
        //https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-integer

        let (min_int, max_int) = (-999_999_999_999_999_i64, 999_999_999_999_999_i64);
        if !(min_int <= value && value <= max_int) {
            return Err("serialize_integer: integer is out of range");
        }
        output.push_str(&value.to_string());
        Ok(())
    }

    fn serialize_decimal(value: Decimal, output: &mut String) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-decimal

        let integer_comp_length = 12;
        let fraction_length = 3;

        let decimal = value.round_dp(fraction_length);
        let int_comp = decimal.trunc();
        let fract_comp = decimal.fract();

        if int_comp.abs().to_string().len() > integer_comp_length {
            return Err("serialize_decimal: integer component > 12 digits");
        }

        if fract_comp.is_zero() {
            output.push_str(&int_comp.to_string());
            output.push('.');
            output.push('0');
        } else {
            output.push_str(&decimal.to_string());
        }

        Ok(())
    }

    fn serialize_string(value: &str, output: &mut String) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-integer

        if !value.is_ascii() {
            return Err("serialize_string: non-ascii character");
        }

        let vchar_or_sp = |char| char == '\x7f' || (char >= '\x00' && char <= '\x1f');
        if value.chars().any(vchar_or_sp) {
            return Err("serialize_string: not a visible character");
        }

        output.push('\"');
        for char in value.chars() {
            if char == '\\' || char == '\"' {
                output.push('\\');
            }
            output.push(char);
        }
        output.push('\"');

        Ok(())
    }

    fn serialize_token(value: &str, output: &mut String) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-token

        if !value.is_ascii() {
            return Err("serialize_string: non-ascii character");
        }

        let mut chars = value.chars();
        if let Some(char) = chars.next() {
            if !(char.is_ascii_alphabetic() || char == '*') {
                return Err("serialise_token: first character is not ALPHA or '*'");
            }
        }

        if chars
            .clone()
            .any(|c| !(utils::is_tchar(c) || c == ':' || c == '/'))
        {
            return Err("serialise_token: disallowed character");
        }

        output.push_str(value);
        Ok(())
    }

    fn serialize_byte_sequence(value: &[u8], output: &mut String) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-binary

        output.push(':');
        let encoded = BASE64.encode(value.as_ref());
        output.push_str(&encoded);
        output.push(':');
        Ok(())
    }

    fn serialize_bool(value: bool, output: &mut String) -> SerializerResult<()> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-boolean

        let val = if value { "?1" } else { "?0" };
        output.push_str(val);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rust_decimal::prelude::FromStr;
    use std::error::Error;
    use std::iter::FromIterator;

    #[test]
    fn serialize_header_empty_dict() -> Result<(), Box<dyn Error>> {
        let dict_header = Header::Dictionary(Dictionary::new());
        assert_eq!("", Serializer::serialize(&dict_header)?);
        Ok(())
    }

    #[test]
    fn serialize_header_empty_list() -> Result<(), Box<dyn Error>> {
        let list_header = Header::List(List::new());
        assert_eq!("", Serializer::serialize(&list_header)?);
        Ok(())
    }

    #[test]
    fn serialize_header_list_mixed_members_with_params() -> Result<(), Box<dyn Error>> {
        let item1 = Item(Decimal::from_str("42.4568")?.into(), Parameters::new());
        let item2_param =
            Parameters::from_iter(vec![("itm2_p".to_owned(), BareItem::Boolean(true))]);
        let item2 = Item(17.into(), item2_param);

        let inner_list_item1_param =
            Parameters::from_iter(vec![("in1_p".to_owned(), BareItem::Boolean(false))]);
        let inner_list_item1 = Item(BareItem::String("str1".to_owned()), inner_list_item1_param);
        let inner_list_item2_param = Parameters::from_iter(vec![(
            "in2_p".to_owned(),
            BareItem::String("valu\\e".to_owned()),
        )]);
        let inner_list_item2 = Item(BareItem::Token("str2".to_owned()), inner_list_item2_param);
        let inner_list_param = Parameters::from_iter(vec![(
            "inner_list_param".to_owned(),
            BareItem::ByteSeq("weather".as_bytes().to_vec()),
        )]);
        let inner_list = InnerList(vec![inner_list_item1, inner_list_item2], inner_list_param);

        let list: List = vec![item1.into(), item2.into(), inner_list.into()];
        let list_header = Header::List(list);
        let expected = "42.457, 17;itm2_p, (\"str1\";in1_p=?0 str2;in2_p=\"valu\\\\e\");inner_list_param=:d2VhdGhlcg==:";
        assert_eq!(expected, Serializer::serialize(&list_header)?);
        Ok(())
    }

    #[test]
    fn serialize_header_errors() -> Result<(), Box<dyn Error>> {
        let disallowed_item = Item(
            BareItem::String("non-ascii text 🐹".into()),
            Parameters::new(),
        );
        assert_eq!(
            Err("serialize_string: non-ascii character"),
            Serializer::serialize(&Header::Item(disallowed_item))
        );

        let disallowed_item = Item(
            Decimal::from_str("12345678912345.123")?.into(),
            Parameters::new(),
        );
        assert_eq!(
            Err("serialize_decimal: integer component > 12 digits"),
            Serializer::serialize(&Header::Item(disallowed_item))
        );

        let param_with_disallowed_key = Parameters::from_iter(vec![("_key".to_owned(), 13.into())]);
        let disallowed_item = Item(12.into(), param_with_disallowed_key);
        assert_eq!(
            Err("serialize_key: first character is not lcalpha or '*'"),
            Serializer::serialize(&Header::Item(disallowed_item))
        );
        Ok(())
    }

    #[test]
    fn serialize_item_byteseq_with_param() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        let item_param = ("a".to_owned(), BareItem::Token("*ab_1".into()));
        let item_param = Parameters::from_iter(vec![item_param]);
        let item = Item(BareItem::ByteSeq("parser".as_bytes().to_vec()), item_param);
        Serializer::serialize_item(&item, &mut buf)?;
        assert_eq!(":cGFyc2Vy:;a=*ab_1", &buf);
        Ok(())
    }

    #[test]
    fn serialize_item_without_params() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        let item = Item(1.into(), Parameters::new());
        Serializer::serialize_item(&item, &mut buf)?;
        assert_eq!("1", &buf);
        Ok(())
    }

    #[test]
    fn serialize_item_with_bool_true_param() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        let param = Parameters::from_iter(vec![("a".to_owned(), BareItem::Boolean(true))]);
        let item = Item(Decimal::from_str("12.35")?.into(), param);
        Serializer::serialize_item(&item, &mut buf)?;
        assert_eq!("12.35;a", &buf);
        Ok(())
    }

    #[test]
    fn serialize_item_with_token_param() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        let param =
            Parameters::from_iter(vec![("a1".to_owned(), BareItem::Token("*tok".to_owned()))]);
        let item = Item(BareItem::String("12.35".to_owned()), param);
        Serializer::serialize_item(&item, &mut buf)?;
        assert_eq!("\"12.35\";a1=*tok", &buf);
        Ok(())
    }

    #[test]
    fn serialize_integer() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        Serializer::serialize_integer(-12, &mut buf)?;
        assert_eq!("-12", &buf);

        buf.clear();
        Serializer::serialize_integer(0, &mut buf)?;
        assert_eq!("0", &buf);

        buf.clear();
        Serializer::serialize_integer(999_999_999_999_999, &mut buf)?;
        assert_eq!("999999999999999", &buf);

        buf.clear();
        Serializer::serialize_integer(-999_999_999_999_999, &mut buf)?;
        assert_eq!("-999999999999999", &buf);
        Ok(())
    }

    #[test]
    fn serialize_integer_errors() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        assert_eq!(
            Err("serialize_integer: integer is out of range"),
            Serializer::serialize_integer(1_000_000_000_000_000, &mut buf)
        );

        buf.clear();
        assert_eq!(
            Err("serialize_integer: integer is out of range"),
            Serializer::serialize_integer(-1_000_000_000_000_000, &mut buf)
        );
        Ok(())
    }

    #[test]
    fn serialize_decimal() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        Serializer::serialize_decimal(Decimal::from_str("-99.1346897")?, &mut buf)?;
        assert_eq!("-99.135", &buf);

        buf.clear();
        Serializer::serialize_decimal(Decimal::from_str("-1.00")?, &mut buf)?;
        assert_eq!("-1.0", &buf);

        buf.clear();
        Serializer::serialize_decimal(
            Decimal::from_str("-00000000000000000000000099.1346897")?,
            &mut buf,
        )?;
        assert_eq!("-99.135", &buf);

        buf.clear();
        Serializer::serialize_decimal(Decimal::from_str("100.13")?, &mut buf)?;
        assert_eq!("100.13", &buf);

        buf.clear();
        Serializer::serialize_decimal(Decimal::from_str("-100.130")?, &mut buf)?;
        assert_eq!("-100.130", &buf);

        buf.clear();
        Serializer::serialize_decimal(Decimal::from_str("-137.0")?, &mut buf)?;
        assert_eq!("-137.0", &buf);

        buf.clear();
        Serializer::serialize_decimal(Decimal::from_str("137121212112.123")?, &mut buf)?;
        assert_eq!("137121212112.123", &buf);

        buf.clear();
        Serializer::serialize_decimal(Decimal::from_str("137121212112.1238")?, &mut buf)?;
        assert_eq!("137121212112.124", &buf);
        Ok(())
    }

    #[test]
    fn serialize_decimal_errors() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        assert_eq!(
            Err("serialize_decimal: integer component > 12 digits"),
            Serializer::serialize_decimal(Decimal::from_str("1371212121121.1")?, &mut buf)
        );
        Ok(())
    }

    #[test]
    fn serialize_string() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        Serializer::serialize_string("1.1 text", &mut buf)?;
        assert_eq!("\"1.1 text\"", &buf);

        buf.clear();
        Serializer::serialize_string("hello \"name\"", &mut buf)?;
        assert_eq!("\"hello \\\"name\\\"\"", &buf);

        buf.clear();
        Serializer::serialize_string("something\\nothing", &mut buf)?;
        assert_eq!("\"something\\\\nothing\"", &buf);

        buf.clear();
        Serializer::serialize_string("", &mut buf)?;
        assert_eq!("\"\"", &buf);

        buf.clear();
        Serializer::serialize_string(" ", &mut buf)?;
        assert_eq!("\" \"", &buf);

        buf.clear();
        Serializer::serialize_string("    ", &mut buf)?;
        assert_eq!("\"    \"", &buf);
        Ok(())
    }

    #[test]
    fn serialize_string_errors() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        assert_eq!(
            Err("serialize_string: not a visible character"),
            Serializer::serialize_string("text \x00", &mut buf)
        );

        assert_eq!(
            Err("serialize_string: not a visible character"),
            Serializer::serialize_string("text \x1f", &mut buf)
        );
        assert_eq!(
            Err("serialize_string: not a visible character"),
            Serializer::serialize_string("text \x7f", &mut buf)
        );
        assert_eq!(
            Err("serialize_string: non-ascii character"),
            Serializer::serialize_string("рядок", &mut buf)
        );
        Ok(())
    }

    #[test]
    fn serialize_token() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        Serializer::serialize_token("*", &mut buf)?;
        assert_eq!("*", &buf);

        buf.clear();
        Serializer::serialize_token("abc", &mut buf)?;
        assert_eq!("abc", &buf);

        buf.clear();
        Serializer::serialize_token("abc:de", &mut buf)?;
        assert_eq!("abc:de", &buf);

        buf.clear();
        Serializer::serialize_token("smth/#!else", &mut buf)?;
        assert_eq!("smth/#!else", &buf);
        Ok(())
    }

    #[test]
    fn serialize_token_errors() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        assert_eq!(
            Err("serialise_token: first character is not ALPHA or '*'"),
            Serializer::serialize_token("#some", &mut buf)
        );
        assert_eq!(
            Err("serialise_token: disallowed character"),
            Serializer::serialize_token("s ", &mut buf)
        );
        assert_eq!(
            Err("serialise_token: disallowed character"),
            Serializer::serialize_token("abc:de\t", &mut buf)
        );
        Ok(())
    }

    #[test]
    fn serialize_byte_sequence() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        Serializer::serialize_byte_sequence("hello".as_bytes(), &mut buf)?;
        assert_eq!(":aGVsbG8=:", &buf);

        buf.clear();
        Serializer::serialize_byte_sequence("test_encode".as_bytes(), &mut buf)?;
        assert_eq!(":dGVzdF9lbmNvZGU=:", &buf);

        buf.clear();
        Serializer::serialize_byte_sequence("".as_bytes(), &mut buf)?;
        assert_eq!("::", &buf);

        buf.clear();
        Serializer::serialize_byte_sequence("pleasure.".as_bytes(), &mut buf)?;
        assert_eq!(":cGxlYXN1cmUu:", &buf);

        buf.clear();
        Serializer::serialize_byte_sequence("leasure.".as_bytes(), &mut buf)?;
        assert_eq!(":bGVhc3VyZS4=:", &buf);

        buf.clear();
        Serializer::serialize_byte_sequence("easure.".as_bytes(), &mut buf)?;
        assert_eq!(":ZWFzdXJlLg==:", &buf);

        buf.clear();
        Serializer::serialize_byte_sequence("asure.".as_bytes(), &mut buf)?;
        assert_eq!(":YXN1cmUu:", &buf);

        buf.clear();
        Serializer::serialize_byte_sequence("sure.".as_bytes(), &mut buf)?;
        assert_eq!(":c3VyZS4=:", &buf);

        Ok(())
    }

    #[test]
    fn serialize_bool() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        Serializer::serialize_bool(true, &mut buf)?;
        assert_eq!("?1", &buf);

        buf.clear();
        Serializer::serialize_bool(false, &mut buf)?;
        assert_eq!("?0", &buf);
        Ok(())
    }

    #[test]
    fn serialize_params_bool() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        let input = Parameters::from_iter(vec![
            ("*b".to_owned(), BareItem::Boolean(true)),
            ("a.a".to_owned(), BareItem::Boolean(true)),
        ]);

        Serializer::serialize_parameters(&input, &mut buf)?;
        assert_eq!(";*b;a.a", &buf);
        Ok(())
    }

    #[test]
    fn serialize_params_string() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        let input = Parameters::from_iter(vec![(
            "b".to_owned(),
            BareItem::String("param_val".to_owned()),
        )]);
        Serializer::serialize_parameters(&input, &mut buf)?;
        assert_eq!(";b=\"param_val\"", &buf);
        Ok(())
    }

    #[test]
    fn serialize_params_numbers() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        let input = Parameters::from_iter(vec![
            ("key1".to_owned(), Decimal::from_str("746.15")?.into()),
            ("key2".to_owned(), 11111.into()),
        ]);
        Serializer::serialize_parameters(&input, &mut buf)?;
        assert_eq!(";key1=746.15;key2=11111", &buf);
        Ok(())
    }

    #[test]
    fn serialize_params_mixed_types() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        let input = Parameters::from_iter(vec![
            ("key1".to_owned(), BareItem::Boolean(false)),
            ("key2".to_owned(), Decimal::from_str("1354.091878")?.into()),
        ]);
        Serializer::serialize_parameters(&input, &mut buf)?;
        assert_eq!(";key1=?0;key2=1354.092", &buf);
        Ok(())
    }

    #[test]
    fn serialize_key() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();
        Serializer::serialize_key("*a_fg", &mut buf)?;
        assert_eq!("*a_fg", &buf);

        buf.clear();
        Serializer::serialize_key("*a_fg*", &mut buf)?;
        assert_eq!("*a_fg*", &buf);

        buf.clear();
        Serializer::serialize_key("key1", &mut buf)?;
        assert_eq!("key1", &buf);

        buf.clear();
        Serializer::serialize_key("ke-y.1", &mut buf)?;
        assert_eq!("ke-y.1", &buf);

        Ok(())
    }

    #[test]
    fn serialize_key_erros() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        assert_eq!(
            Err("serialize_key: disallowed character in input"),
            Serializer::serialize_key("AND", &mut buf)
        );
        assert_eq!(
            Err("serialize_key: first character is not lcalpha or '*'"),
            Serializer::serialize_key("_key", &mut buf)
        );
        assert_eq!(
            Err("serialize_key: first character is not lcalpha or '*'"),
            Serializer::serialize_key("7key", &mut buf)
        );
        Ok(())
    }

    #[test]
    fn serialize_list_of_items_and_inner_list() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        let item1 = Item(12.into(), Parameters::new());
        let item2 = Item(14.into(), Parameters::new());
        let item3 = Item(BareItem::Token("a".to_owned()), Parameters::new());
        let item4 = Item(BareItem::Token("b".to_owned()), Parameters::new());
        let inner_list_param = Parameters::from_iter(vec![(
            "param".to_owned(),
            BareItem::String("param_value_1".to_owned()),
        )]);
        let inner_list = InnerList(vec![item3, item4], inner_list_param);
        let input: List = vec![item1.into(), item2.into(), inner_list.into()];

        Serializer::serialize_list(&input, &mut buf)?;
        assert_eq!("12, 14, (a b);param=\"param_value_1\"", &buf);
        Ok(())
    }

    #[test]
    fn serialize_list_of_lists() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        let item1 = Item(1.into(), Parameters::new());
        let item2 = Item(2.into(), Parameters::new());
        let item3 = Item(42.into(), Parameters::new());
        let item4 = Item(43.into(), Parameters::new());
        let inner_list_1 = InnerList(vec![item1, item2], Parameters::new());
        let inner_list_2 = InnerList(vec![item3, item4], Parameters::new());
        let input: List = vec![inner_list_1.into(), inner_list_2.into()];

        Serializer::serialize_list(&input, &mut buf)?;
        assert_eq!("(1 2), (42 43)", &buf);
        Ok(())
    }

    #[test]
    fn serialize_list_with_bool_item_and_bool_params() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        let item1_params = Parameters::from_iter(vec![
            ("a".to_owned(), BareItem::Boolean(true)),
            ("b".to_owned(), BareItem::Boolean(false)),
        ]);
        let item1 = Item(BareItem::Boolean(false), item1_params);
        let item2 = Item(BareItem::Token("cde_456".to_owned()), Parameters::new());

        let input: List = vec![item1.into(), item2.into()];
        Serializer::serialize_list(&input, &mut buf)?;
        assert_eq!("?0;a;b=?0, cde_456", &buf);
        Ok(())
    }

    #[test]
    fn serialize_dictionary_with_params() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        let item1_params = Parameters::from_iter(vec![
            ("a".to_owned(), 1.into()),
            ("b".to_owned(), BareItem::Boolean(true)),
        ]);
        let item2_params = Parameters::new();
        let item3_params = Parameters::from_iter(vec![
            ("q".to_owned(), BareItem::Boolean(false)),
            ("r".to_owned(), BareItem::String("+w".to_owned())),
        ]);

        let item1 = Item(123.into(), item1_params);
        let item2 = Item(456.into(), item2_params);
        let item3 = Item(789.into(), item3_params);

        let input = Dictionary::from_iter(vec![
            ("abc".to_owned(), item1.into()),
            ("def".to_owned(), item2.into()),
            ("ghi".to_owned(), item3.into()),
        ]);

        Serializer::serialize_dictionary(&input, &mut buf)?;
        assert_eq!("abc=123;a=1;b, def=456, ghi=789;q=?0;r=\"+w\"", &buf);
        Ok(())
    }

    #[test]
    fn serialize_dict_empty_member_value() -> Result<(), Box<dyn Error>> {
        let mut buf = String::new();

        let inner_list = InnerList(vec![], Parameters::new());
        let input = Dictionary::from_iter(vec![("a".to_owned(), inner_list.into())]);
        Serializer::serialize_dictionary(&input, &mut buf)?;
        assert_eq!("a=()", &buf);
        Ok(())
    }
}
