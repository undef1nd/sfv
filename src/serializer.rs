use crate::parser::*;
use crate::utils;
use data_encoding::BASE64;
use rust_decimal::Decimal;

type SerializerResult<T> = Result<T, &'static str>;

struct Serializer;

impl Serializer {
    fn serialize(header: &Header) -> SerializerResult<String> {
        // match and call respective func
        Ok("1".to_owned())
    }

    fn serialize_item(item: &Item, output: &mut String) -> SerializerResult<()> {
        Self::serialize_bare_item(&item.0, output);
        Self::serialize_parameters(&item.1, output);
        Ok(())
    }

    fn serialize_bare_item(bare_item: &BareItem, output: &mut String) -> SerializerResult<()> {
        Ok(match bare_item {
            BareItem::Boolean(value) => Self::serialize_bool(*value, output)?,
            BareItem::String(value) => Self::serialize_string(value, output)?,
            BareItem::ByteSeq(value) => Self::serialize_byte_sequence(value, output)?,
            BareItem::Token(value) => Self::serialize_token(value, output)?,
            BareItem::Number(Num::Integer(value)) => Self::serialize_integer(*value, output)?,
            BareItem::Number(Num::Decimal(value)) => Self::serialize_decimal(*value, output)?,
        })
    }

    fn serialize_parameters(value: &Parameters, output: &mut String) -> SerializerResult<()> {
        for (param_name, param_value) in value.iter() {
            output.push(';');
            &Self::serialize_key(param_name, output)?;

            if param_value != &BareItem::Boolean(true) {
                output.push('=');
                &Self::serialize_bare_item(param_value, output)?;
            }
        }
        Ok(())
    }

    fn serialize_key(value: &str, output: &mut String) -> SerializerResult<()> {
        let disallowed_chars =
            |c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || "_-*.".contains(c));

        if value.chars().any(disallowed_chars) {
            return Err("serialize_key: disallowed character in input");
        }

        if let Some(char) = value.chars().next() {
            if !(char.is_ascii_lowercase() || char == '*') {
                return Err("serialize_key: first character is not lcalpha or '*'");
            }
        }
        output.push_str(value);
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
        let int_comp = decimal.trunc().abs();

        if int_comp.to_string().len() > integer_comp_length {
            return Err("serialize_decimal: integer component > 12 digits");
        }
        output.push_str(&decimal.to_string());
        Ok(())
    }

    fn serialize_string(value: &str, output: &mut String) -> SerializerResult<()> {
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

        output.push_str(value.into());
        Ok(())
    }

    fn serialize_byte_sequence(value: &[u8], output: &mut String) -> SerializerResult<()> {
        output.push(':');
        let encoded = BASE64.encode(value.as_ref());
        output.push_str(&encoded);
        output.push(':');
        Ok(())
    }

    fn serialize_bool(value: bool, output: &mut String) -> SerializerResult<()> {
        let val = match value {
            true => "?1",
            false => "?0",
        };
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

    // #[test]
    // fn serialize() -> Result<(), Box<dyn Error>> {
    //    assert_eq!(1, 1);
    //     Ok(())
    // }

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
        Serializer::serialize_decimal(Decimal::from_str("-99.1346897")?, &mut buf);
        assert_eq!("-99.135", &buf);

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
}
