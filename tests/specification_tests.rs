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

    // Rest of the tests, including allowed to fail, must have expected value
    let expected_value = test_case
        .expected
        .as_ref()
        .ok_or("expected value is not specified")?;
    let expected_header = match test_case.header_type.as_str() {
        "item" => {
            let item = get_item_struct(expected_value)?;
            Header::Item(item)
        }
        "list" => unimplemented!(),
        "dictionary" => unimplemented!(),
        _ => return Err("unknown header_type value".into()),
    };
    assert_eq!(expected_header, actual_result?);
    Ok(())
}

// fn get_item_struct(expected_struct_value: &Value) -> Result<Header, Box<dyn Error>>{
fn get_item_struct(expected_struct_value: &Value) -> Result<Item, Box<dyn Error>> {
    let expected_array = expected_struct_value
        .as_array()
        .ok_or("expected value is not array")?;
    if expected_array.len() != 2 {
        return Err("Not an item".into());
    }

    let bare_item_val = &expected_array[0];
    let params_val = &expected_array[1];
    let bare_item = get_bare_item(bare_item_val)?;

    let mut parameters = Parameters::new();
    for (key, val) in params_val.as_object().ok_or("params value is not object")? {
        let itm = get_bare_item(val)?;
        parameters.insert(key.clone(), itm);
    }

    Ok(Item {
        bare_item,
        parameters,
    })
}

fn get_bare_item(bare_item: &Value) -> Result<BareItem, Box<dyn Error>> {
    match bare_item {
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
        _ => return Err("unknown bare_item value".into()),
    }
}

#[test]
fn run_specification_tests() -> Result<(), Box<dyn Error>> {
    let test_suites_dir: PathBuf = env::current_dir()?.join("tests").join("test_suites");
    for file in fs::read_dir(test_suites_dir)? {
        run_test_suite(file?.path())?
    }

    // let test_cases: Vec<TestData> = serde_json::from_str(include_str!("test_suites/binary.json"))?;
    //
    // for case in test_cases.iter() {
    //     handle_test_case(case)?
    // }

    Ok(())
}

fn run_test_suite(tests_file: PathBuf) -> Result<(), Box<dyn Error>> {
    let test_cases: Vec<TestData> = serde_json::from_reader(fs::File::open(tests_file)?)?;
    for test_data in test_cases.iter() {
        handle_test_case(test_data)?;
    }
    Ok(())
}
