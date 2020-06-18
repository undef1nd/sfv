use data_encoding::BASE32;
use rust_decimal::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use std::path::PathBuf;
use std::{env, fs};
use structured_headers::parser::*;
use structured_headers::serializer::Serializer;

#[derive(Debug, PartialEq, Deserialize)]
struct TestData {
    name: String,
    raw: Option<Vec<String>>,
    header_type: String,
    expected: Option<Value>,
    can_fail: Option<bool>,
    must_fail: Option<bool>,
    canonical: Option<Vec<String>>,
}

fn run_test_case(test_case: &TestData) -> Result<(), Box<dyn Error>> {
    println!("- {}", &test_case.name);

    let input = test_case
        .raw
        .as_ref()
        .ok_or("raw value is not specified")?
        .join(", ");

    let actual_header = match test_case.header_type.as_str() {
        "dictionary" => {
            let res = Parser::parse_dict_header(input.as_bytes());
            println!("{:?}, {:?}", test_case.must_fail, res.is_err());
            if let Some(true) = test_case.must_fail {
                assert!(res.is_err());
                return Ok(());
            };
            Header::Dictionary(res?)
        }
        "list" => {
            let res = Parser::parse_list_header(input.as_bytes());
            if let Some(true) = test_case.must_fail {
                assert!(res.is_err());
                return Ok(());
            };
            Header::List(res?)
        }
        "item" => {
            let res = Parser::parse_item_header(input.as_bytes());
            if let Some(true) = test_case.must_fail {
                assert!(res.is_err());
                return Ok(());
            };
            Header::Item(res?)
        }
        _ => return Err("unexpected header type".into()),
    };

    let expected_header = build_expected_header(test_case)?;

    // Test parsing
    match (&actual_header, &expected_header) {
        (Header::Dictionary(val1), Header::Dictionary(val2)) => {
            assert!(val1.iter().eq(val2.iter()));
        }
        (Header::List(val1), Header::List(val2)) => {
            assert!(val1.iter().eq(val2.iter()));
        }
        (_, _) => {
            assert_eq!(expected_header, actual_header);
        }
    }

    // Test serialization
    if let Some(canonical_val) = &test_case.canonical {
        let expected_serialized = canonical_val.join("");
        assert_eq!(expected_serialized, Serializer::serialize(&actual_header)?)
    }
    Ok(())
}

fn run_test_case_serialzation_only(test_case: &TestData) -> Result<(), Box<dyn Error>> {
    let expected_header = build_expected_header(test_case)?;
    let actual_result = Serializer::serialize(&expected_header);

    if let Some(true) = test_case.must_fail {
        assert!(actual_result.is_err());
        return Ok(());
    }

    // Test serialization
    if let Some(canonical_val) = &test_case.canonical {
        let expected_serialized = canonical_val.join("");
        assert_eq!(expected_serialized, actual_result?);
    }

    Ok(())
}

fn build_expected_header(test_case: &TestData) -> Result<Header, Box<dyn Error>> {
    let expected_value = test_case
        .expected
        .as_ref()
        .ok_or("test expected value is not specified")?;

    // Build expected Header from serde Value
    match test_case.header_type.as_str() {
        "item" => {
            let item = build_item(expected_value)?;
            Ok(Header::Item(item))
        }
        "list" => {
            let list = build_list(expected_value)?;
            Ok(Header::List(list))
        }
        "dictionary" => {
            let dict = build_dict(expected_value)?;
            Ok(Header::Dictionary(dict))
        }
        _ => return Err("unknown header_type value".into()),
    }
}

fn build_list_or_item(member: &Value) -> Result<ListEntry, Box<dyn Error>> {
    let member_as_array = member.as_array().ok_or("list_or_item value is not array")?;

    // If member is an array of arrays, then it represents InnerList, otherwise it's an Item
    let list_entry: ListEntry = if member_as_array[0].is_array() {
        build_inner_list(member)?.into()
    } else {
        build_item(member)?.into()
    };
    Ok(list_entry)
}

fn build_dict(expected_value: &Value) -> Result<Dictionary, Box<dyn Error>> {
    let expected_as_map = expected_value
        .as_object()
        .ok_or("expected value is not object")?;

    let mut dict = Dictionary::new();
    for (member_name, member_value) in expected_as_map.iter() {
        let item_or_inner_list: ListEntry = build_list_or_item(member_value)?;
        dict.insert(member_name.clone(), item_or_inner_list);
    }
    Ok(dict)
}

fn build_list(expected_value: &Value) -> Result<List, Box<dyn Error>> {
    let expected_as_array = expected_value
        .as_array()
        .ok_or("expected value is not array")?;

    let mut list_items: Vec<ListEntry> = vec![];
    for member in expected_as_array.iter() {
        let item_or_inner_list: ListEntry = build_list_or_item(member)?;
        list_items.push(item_or_inner_list);
    }
    Ok(list_items)
}

fn build_inner_list(inner_list_value: &Value) -> Result<InnerList, Box<dyn Error>> {
    // Inner list contains array of items and map of parameters:
    // inner list = [[items], {params}]
    // each item is an array itself: item = [bare_item, {params}]
    let inner_list_as_array = inner_list_value
        .as_array()
        .ok_or("inner list is not array")?;

    let inner_list_items = &inner_list_as_array[0];
    let inner_list_params = &inner_list_as_array[1];

    let mut items = vec![];
    for item_value in inner_list_items
        .as_array()
        .ok_or("inner list items value is not array")?
        .iter()
    {
        items.push(build_item(item_value)?);
    }

    let parameters = build_parameters(inner_list_params)?;

    Ok(InnerList(items, parameters))
}

fn build_item(expected_value: &Value) -> Result<Item, Box<dyn Error>> {
    // item = [bare_item, {params}]
    let expected_array = expected_value
        .as_array()
        .ok_or("expected value is not array")?;

    // Item array must contain 2 members only
    if expected_array.len() != 2 {
        return Err("Not an item".into());
    }

    let bare_item_val = &expected_array[0];
    let params_val = &expected_array[1];
    let bare_item = build_bare_item(bare_item_val)?;
    let parameters = build_parameters(params_val)?;

    Ok(Item(bare_item, parameters))
}

fn build_bare_item(bare_item_value: &Value) -> Result<BareItem, Box<dyn Error>> {
    match bare_item_value {
        bare_item if bare_item.is_i64() => Ok(BareItem::Number(Num::Integer(
            bare_item.as_i64().ok_or("bare_item value is not i64")?,
        ))),
        bare_item if bare_item.is_f64() => {
            Ok(Decimal::from_str(&serde_json::to_string(bare_item)?)?.into())
        }
        bare_item if bare_item.is_boolean() => Ok(BareItem::Boolean(
            bare_item.as_bool().ok_or("bare_item value is not bool")?,
        )),
        bare_item if bare_item.is_string() => Ok(BareItem::String(
            bare_item
                .as_str()
                .ok_or("bare_item value is not str")?
                .clone()
                .to_owned(),
        )),
        bare_item if (bare_item.is_object() && bare_item["__type"] == "token") => {
            Ok(BareItem::Token(
                bare_item["value"]
                    .as_str()
                    .ok_or("bare_item value is not str")?
                    .clone()
                    .to_owned(),
            ))
        }
        bare_item if (bare_item.is_object() && bare_item["__type"] == "binary") => {
            let str_val = bare_item["value"]
                .as_str()
                .ok_or("bare_item value is not str")?
                .clone();
            Ok(BareItem::ByteSeq(BASE32.decode(str_val.as_bytes())?))
        }
        _ => Err("unknown bare_item value".into()),
    }
}

fn build_parameters(params_value: &Value) -> Result<Parameters, Box<dyn Error>> {
    let mut parameters = Parameters::new();
    for (key, val) in params_value
        .as_object()
        .ok_or("params value is not object")?
    {
        let itm = build_bare_item(val)?;
        parameters.insert(key.clone(), itm);
    }
    Ok(parameters)
}

fn run_test_suite(tests_file: PathBuf, is_serialization: bool) -> Result<(), Box<dyn Error>> {
    let test_cases: Vec<TestData> = serde_json::from_reader(fs::File::open(tests_file)?)?;
    for test_data in test_cases.iter() {
        if is_serialization {
            run_test_case_serialzation_only(test_data)?;
        } else {
            run_test_case(test_data)?;
        }
    }
    Ok(())
}

#[test]
fn run_spec_parse_serialize_tests() -> Result<(), Box<dyn Error>> {
    let test_suites_dir: PathBuf = env::current_dir()?.join("tests").join("spec_tests");
    let json_files = fs::read_dir(test_suites_dir)?
        .filter_map(Result::ok)
        .filter(|fp| fp.path().extension().unwrap_or_default() == "json");

    for file_path in json_files {
        println!("\n## Test suite file: {:?}\n", &file_path.file_name());
        run_test_suite(file_path.path(), false)?
    }
    Ok(())
}

#[test]
fn run_spec_serialize_only_tests() -> Result<(), Box<dyn Error>> {
    let test_suites_dir: PathBuf = env::current_dir()?
        .join("tests")
        .join("spec_tests")
        .join("serialisation-tests");
    let json_files = fs::read_dir(test_suites_dir)?
        .filter_map(Result::ok)
        .filter(|fp| fp.path().extension().unwrap_or_default() == "json");

    for file_path in json_files {
        println!(
            "\n## Serialization test suite file: {:?}\n",
            &file_path.file_name()
        );
        run_test_suite(file_path.path(), true)?
    }
    Ok(())
}
