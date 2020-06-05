use data_encoding::BASE32;
use rust_decimal::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use std::error::Error;
use std::path::PathBuf;
use std::{env, fs};
use structured_headers::parser::*;

#[derive(Debug, PartialEq, Deserialize)]
struct TestData {
    name: String,
    raw: Vec<String>,
    header_type: String,
    expected: Option<Value>,
    can_fail: Option<bool>,
    must_fail: Option<bool>,
    canonical: Option<Vec<String>>,
}

fn handle_test_case(test_case: &TestData) -> Result<(), Box<dyn Error>> {
    println!("#### {}", &test_case.name);
    let input = test_case.raw.join(", ");
    let actual_result = Parser::parse(input.as_bytes(), &test_case.header_type);

    // Check test that must fail actually fails
    if let Some(true) = test_case.must_fail {
        assert!(actual_result.is_err());
        return Ok(());
    }

    // Some tests are allowed to fail
    if actual_result.is_err() {
        if let Some(true) = test_case.can_fail {
            return Ok(());
        }
    }

    // Rest of the tests, including those allowed to fail, must have expected value
    let expected_value = test_case
        .expected
        .as_ref()
        .ok_or("expected value is not specified")?;

    // Build Header struct from test case expected value
    let expected_header = match test_case.header_type.as_str() {
        "item" => {
            let item = get_item_struct(expected_value)?;
            Header::Item(item)
        }
        "list" => {
            let list = get_list_struct(expected_value)?;
            Header::List(list)
        }
        "dictionary" => unimplemented!(),
        _ => return Err("unknown header_type value".into()),
    };

    assert_eq!(expected_header, actual_result?);
    Ok(())
}

fn get_list_struct(expected_value: &Value) -> Result<List, Box<dyn Error>> {
    let expected_array = expected_value
        .as_array()
        .ok_or("expected value is not array")?;

    let mut list_items: Vec<ListEntry> = vec![];
    for member in expected_array.iter() {
        // if first item in member is array, then member must be parsed into InnerList
        // otherwise - into Item
        if member.as_array().unwrap()[0].is_array() {
            let list_entry = get_inner_list(member)?;
            list_items.push(list_entry.into());
        } else {
            let item = get_item_struct(member)?;
            list_items.push(item.into());
        }
    }
    Ok(List { items: list_items })
}

fn get_inner_list(inner_list_value: &Value) -> Result<InnerList, Box<dyn Error>> {
    // inner list contains array of items and indexmap of parameters: inner_list = [[item1, item2], {params}]
    // each item is an array itself: item = [bare_item, {params}]
    let inner_list = inner_list_value
        .as_array()
        .ok_or("inner list is not array")?;

    let inner_list_items = &inner_list[0];
    let inner_list_params = &inner_list[1];

    let mut items = vec![];
    for item_value in inner_list_items
        .as_array()
        .ok_or("inner list items value is not array")?
        .iter()
    {
        items.push(get_item_struct(item_value)?);
    }

    let parameters = get_parameters(inner_list_params)?;

    Ok(InnerList { items, parameters })
}

fn get_item_struct(expected_value: &Value) -> Result<Item, Box<dyn Error>> {
    let expected_array = expected_value
        .as_array()
        .ok_or("expected value is not array")?;

    // Item array must contain 2 members only
    if expected_array.len() != 2 {
        return Err("Not an item".into());
    }

    let bare_item_val = &expected_array[0];
    let params_val = &expected_array[1];
    let bare_item = get_bare_item(bare_item_val)?;
    let parameters = get_parameters(params_val)?;

    Ok(Item {
        bare_item,
        parameters,
    })
}

fn get_bare_item(bare_item_value: &Value) -> Result<BareItem, Box<dyn Error>> {
    // Guess kind of BareItem represented by serde Value
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

fn get_parameters(params_value: &Value) -> Result<Parameters, Box<dyn Error>> {
    let mut parameters = Parameters::new();
    for (key, val) in params_value
        .as_object()
        .ok_or("params value is not object")?
    {
        let itm = get_bare_item(val)?;
        parameters.insert(key.clone(), itm);
    }
    Ok(parameters)
}

#[test]
fn run_specification_tests() -> Result<(), Box<dyn Error>> {
    let test_suites_dir: PathBuf = env::current_dir()?.join("tests").join("test_suites");
    for file in fs::read_dir(test_suites_dir)? {
        run_test_suite(file?.path())?
    }
    Ok(())
}

fn run_test_suite(tests_file: PathBuf) -> Result<(), Box<dyn Error>> {
    let test_cases: Vec<TestData> = serde_json::from_reader(fs::File::open(tests_file)?)?;
    for test_data in test_cases.iter() {
        handle_test_case(test_data)?;
    }
    Ok(())
}
