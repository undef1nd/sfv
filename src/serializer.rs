use crate::parser::*;
use crate::utils;
use rust_decimal::Decimal;

type SerializeResult<T> = Result<T, &'static str>;

struct Serializer;

impl Serializer {
    fn serialize(header: &Header) -> SerializeResult<String> {
        // match and call respective func
        Ok("1".to_owned())
    }

    fn serialize_item(item: &Item) -> SerializeResult<String> {
        let mut output = String::new();
        output.push_str(Self::serialize_bare_item(&item.0)?.as_str());
        output.push_str(Self::serialize_parameters(&item.1)?.as_str());
        Ok(output)
    }

    fn serialize_bare_item(bare_item: &BareItem) -> SerializeResult<String> {
        match bare_item {
            BareItem::Boolean(value) => Self::serialize_bool(*value),
            BareItem::String(value) => Self::serialize_string(value),
            BareItem::ByteSeq(value) => Self::serialize_byte_sequence(value),
            BareItem::Token(value) => Self::serialize_token(value),
            BareItem::Number(Num::Integer(value)) => Self::serialize_integer(*value),
            BareItem::Number(Num::Decimal(value)) => Self::serialize_decimal(*value),
        }
    }

    fn serialize_parameters(value: &Parameters) -> SerializeResult<String> {
        let mut output = String::new();
        for (param_name, param_value) in value.iter() {
            output.push(';');
            output.push_str(&Self::serialize_key(param_name)?);

            if param_value != &BareItem::Boolean(true) {
                output.push('=');
                output.push_str(&Self::serialize_bare_item(param_value)?);
            }
        }
        Ok(output)
    }

    fn serialize_key(value: &str) -> SerializeResult<String> {
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
        Ok(value.to_owned())
    }

    fn serialize_integer(value: i64) -> SerializeResult<String> {
        //https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-integer

        let (min_int, max_int) = (-999_999_999_999_999_i64, 999_999_999_999_999_i64);
        if !(min_int <= value && value <= max_int) {
            return Err("serialize_integer: integer is out of range");
        }

        let output = value.to_string();
        Ok(output)
    }

    fn serialize_decimal(value: Decimal) -> SerializeResult<String> {
        // https://httpwg.org/http-extensions/draft-ietf-httpbis-header-structure.html#ser-decimal
        let integer_comp_length = 12;
        let fraction_length = 3;

        let decimal = value.round_dp(fraction_length);
        let int_comp = decimal.trunc().abs();

        if int_comp.to_string().len() > integer_comp_length {
            return Err("serialize_decimal: integer component > 12 digits");
        }
        Ok(decimal.to_string())
    }

    fn serialize_string(value: &str) -> SerializeResult<String> {
        if !value.is_ascii() {
            return Err("serialize_string: non-ascii character");
        }

        let vchar_or_sp = |char| char == '\x7f' || (char >= '\x00' && char <= '\x1f');
        if value.chars().any(vchar_or_sp) {
            return Err("serialize_string: not a visible character");
        }

        let mut output = String::with_capacity(value.len());
        output.push('\"');
        for char in value.chars() {
            if char == '\\' || char == '\"' {
                output.push('\\');
            }
            output.push(char);
        }
        output.push('\"');

        Ok(output)
    }

    fn serialize_token(value: &str) -> SerializeResult<String> {
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

        Ok(value.to_owned())
    }

    fn serialize_byte_sequence(value: &[u8]) -> SerializeResult<String> {
        let mut output = String::new();
        output.push(':');
        let encoded = data_encoding::BASE64.encode(value.as_ref());
        output.push_str(&encoded);
        output.push(':');
        Ok(output)
    }

    fn serialize_bool(value: bool) -> SerializeResult<String> {
        match value {
            true => Ok("?1".to_owned()),
            false => Ok("?0".to_owned()),
        }
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
        let item = Item(1.into(), Parameters::new());
        assert_eq!("1", Serializer::serialize_item(&item)?);
        Ok(())
    }

    #[test]
    fn serialize_item_with_bool_true_param() -> Result<(), Box<dyn Error>> {
        let param = Parameters::from_iter(vec![("a".to_owned(), BareItem::Boolean(true))]);
        let item = Item(Decimal::from_str("12.35")?.into(), param);
        assert_eq!("12.35;a", Serializer::serialize_item(&item)?);
        Ok(())
    }

    #[test]
    fn serialize_item_with_token_param() -> Result<(), Box<dyn Error>> {
        let param =
            Parameters::from_iter(vec![("a1".to_owned(), BareItem::Token("*tok".to_owned()))]);
        let item = Item(BareItem::String("12.35".to_owned()), param);
        assert_eq!("\"12.35\";a1=*tok", Serializer::serialize_item(&item)?);
        Ok(())
    }

    #[test]
    fn serialize_integer() -> Result<(), Box<dyn Error>> {
        assert_eq!("-12", &Serializer::serialize_integer(-12)?);
        assert_eq!("0", &Serializer::serialize_integer(0)?);
        assert_eq!(
            "999999999999999",
            &Serializer::serialize_integer(999_999_999_999_999)?
        );
        assert_eq!(
            "-999999999999999",
            &Serializer::serialize_integer(-999_999_999_999_999)?
        );
        Ok(())
    }

    fn serialize_integer_errors() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Err("serialize_integer: integer is out of range"),
            Serializer::serialize_integer(1_000_000_000_000_000)
        );
        assert_eq!(
            Err("serialize_integer: integer is out of range"),
            Serializer::serialize_integer(-1_000_000_000_000_000)
        );
        Ok(())
    }

    #[test]
    fn serialize_decimal() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            "-99.135",
            &Serializer::serialize_decimal(Decimal::from_str("-99.1346897")?)?
        );
        assert_eq!(
            "-99.135",
            &Serializer::serialize_decimal(Decimal::from_str(
                "-00000000000000000000000099.1346897"
            )?)?
        );
        assert_eq!(
            "100.13",
            &Serializer::serialize_decimal(Decimal::from_str("100.13")?)?
        );
        assert_eq!(
            "-100.130",
            &Serializer::serialize_decimal(Decimal::from_str("-100.130")?)?
        );
        assert_eq!(
            "-137.0",
            &Serializer::serialize_decimal(Decimal::from_str("-137.0")?)?
        );
        assert_eq!(
            "137121212112.123",
            &Serializer::serialize_decimal(Decimal::from_str("137121212112.123")?)?
        );
        assert_eq!(
            "137121212112.124",
            &Serializer::serialize_decimal(Decimal::from_str("137121212112.1238")?)?
        );
        Ok(())
    }

    #[test]
    fn serialize_decimal_errors() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Err("serialize_decimal: integer component > 12 digits"),
            Serializer::serialize_decimal(Decimal::from_str("1371212121121.1")?)
        );
        Ok(())
    }

    #[test]
    fn serialize_string() -> Result<(), Box<dyn Error>> {
        assert_eq!("\"1.1 text\"", &Serializer::serialize_string("1.1 text")?);
        assert_eq!(
            "\"hello \\\"name\\\"\"",
            &Serializer::serialize_string("hello \"name\"")?
        );
        assert_eq!(
            "\"something\\\\nothing\"",
            &Serializer::serialize_string("something\\nothing")?
        );
        assert_eq!("\"\"", &Serializer::serialize_string("")?);
        assert_eq!("\" \"", &Serializer::serialize_string(" ")?);
        assert_eq!("\"    \"", &Serializer::serialize_string("    ")?);
        Ok(())
    }

    #[test]
    fn serialize_string_errors() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Err("serialize_string: not a visible character"),
            Serializer::serialize_string("text \x00")
        );
        assert_eq!(
            Err("serialize_string: not a visible character"),
            Serializer::serialize_string("text \x1f")
        );
        assert_eq!(
            Err("serialize_string: not a visible character"),
            Serializer::serialize_string("text \x7f")
        );
        assert_eq!(
            Err("serialize_string: non-ascii character"),
            Serializer::serialize_string("рядок")
        );
        Ok(())
    }

    #[test]
    fn serialize_token() -> Result<(), Box<dyn Error>> {
        assert_eq!("*", Serializer::serialize_token("*")?);
        assert_eq!("abc", Serializer::serialize_token("abc")?);
        assert_eq!("abc:de", Serializer::serialize_token("abc:de")?);
        assert_eq!("smth/#!else", Serializer::serialize_token("smth/#!else")?);
        Ok(())
    }

    #[test]
    fn serialize_token_errors() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Err("serialise_token: first character is not ALPHA or '*'"),
            Serializer::serialize_token("#some")
        );
        assert_eq!(
            Err("serialise_token: disallowed character"),
            Serializer::serialize_token("s ")
        );
        assert_eq!(
            Err("serialise_token: disallowed character"),
            Serializer::serialize_token("abc:de\t")
        );
        Ok(())
    }

    #[test]
    fn serialize_byte_sequence() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            ":aGVsbG8=:",
            Serializer::serialize_byte_sequence("hello".as_bytes())?
        );
        assert_eq!(
            ":dGVzdF9lbmNvZGU=:",
            Serializer::serialize_byte_sequence("test_encode".as_bytes())?
        );
        assert_eq!("::", Serializer::serialize_byte_sequence("".as_bytes())?);
        assert_eq!(
            ":cGxlYXN1cmUu:",
            Serializer::serialize_byte_sequence("pleasure.".as_bytes())?
        );
        assert_eq!(
            ":bGVhc3VyZS4=:",
            Serializer::serialize_byte_sequence("leasure.".as_bytes())?
        );
        assert_eq!(
            ":ZWFzdXJlLg==:",
            Serializer::serialize_byte_sequence("easure.".as_bytes())?
        );
        assert_eq!(
            ":YXN1cmUu:",
            Serializer::serialize_byte_sequence("asure.".as_bytes())?
        );
        assert_eq!(
            ":c3VyZS4=:",
            Serializer::serialize_byte_sequence("sure.".as_bytes())?
        );

        Ok(())
    }

    #[test]
    fn serialize_bool() -> Result<(), Box<dyn Error>> {
        assert_eq!("?1", Serializer::serialize_bool(true)?);
        assert_eq!("?0", Serializer::serialize_bool(false)?);
        Ok(())
    }

    #[test]
    fn serialize_params_bool() -> Result<(), Box<dyn Error>> {
        let input = Parameters::from_iter(vec![
            ("*b".to_owned(), BareItem::Boolean(true)),
            ("a.a".to_owned(), BareItem::Boolean(true)),
        ]);
        assert_eq!(";*b;a.a", Serializer::serialize_parameters(&input)?);
        Ok(())
    }

    #[test]
    fn serialize_params_string() -> Result<(), Box<dyn Error>> {
        let input = Parameters::from_iter(vec![(
            "b".to_owned(),
            BareItem::String("param_val".to_owned()),
        )]);
        assert_eq!(
            ";b=\"param_val\"",
            &Serializer::serialize_parameters(&input)?
        );
        Ok(())
    }

    #[test]
    fn serialize_params_numbers() -> Result<(), Box<dyn Error>> {
        let input = Parameters::from_iter(vec![
            ("key1".to_owned(), Decimal::from_str("746.15")?.into()),
            ("key2".to_owned(), 11111.into()),
        ]);
        assert_eq!(
            ";key1=746.15;key2=11111",
            &Serializer::serialize_parameters(&input)?
        );
        Ok(())
    }

    #[test]
    fn serialize_params_mixed_types() -> Result<(), Box<dyn Error>> {
        let input = Parameters::from_iter(vec![
            ("key1".to_owned(), BareItem::Boolean(false)),
            ("key2".to_owned(), Decimal::from_str("1354.091878")?.into()),
        ]);
        assert_eq!(
            ";key1=?0;key2=1354.092",
            &Serializer::serialize_parameters(&input)?
        );
        Ok(())
    }

    #[test]
    fn serialize_key() -> Result<(), Box<dyn Error>> {
        assert_eq!("*a_fg", &Serializer::serialize_key("*a_fg")?);
        assert_eq!("*a_fg*", &Serializer::serialize_key("*a_fg*")?);
        assert_eq!("key1", &Serializer::serialize_key("key1")?);
        assert_eq!("ke-y.1", &Serializer::serialize_key("ke-y.1")?);
        Ok(())
    }

    #[test]
    fn serialize_key_erros() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            Err("serialize_key: disallowed character in input"),
            Serializer::serialize_key("AND")
        );
        assert_eq!(
            Err("serialize_key: first character is not lcalpha or '*'"),
            Serializer::serialize_key("_key")
        );
        assert_eq!(
            Err("serialize_key: first character is not lcalpha or '*'"),
            Serializer::serialize_key("7key")
        );
        Ok(())
    }
}
