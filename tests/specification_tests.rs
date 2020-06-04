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
    // TODO: need to handle can fail
    let input = test_case.raw.join(", ");
    let input_bytes = input.as_bytes();

    // If test must fail, verify it and exit
    if let Some(true) = test_case.must_fail {
        let actual = Parser::parse(input_bytes, &test_case.header_type);
        assert!(actual.is_err());
        return Ok(());
    }

    // Otherwise test must have expected value
    let expected_value = test_case.expected.as_ref().unwrap();
    let expected_header = match test_case.header_type.as_str() {
        "item" => {
            let item = get_item_struct(expected_value)?;
            Header::Item(item)
        }
        "list" => unimplemented!(),
        "dictionary" => unimplemented!(),
        _ => return Err("Unknown header_type value".into()),
    };
    assert_eq!(
        expected_header,
        Parser::parse(input_bytes, &test_case.header_type)?
    );
    Ok(())
}

// fn get_item_struct(expected_struct_value: &Value) -> Result<Header, Box<dyn Error>>{
fn get_item_struct(expected_struct_value: &Value) -> Result<Item, Box<dyn Error>> {
    let expected_array = expected_struct_value.as_array().unwrap();
    if expected_array.len() != 2 {
        return Err("Not an item".into());
    }

    let (bare_item_val, params_val) = (&expected_array[0], &expected_array[1]);
    let bare_item = get_bare_item(bare_item_val)?;

    let mut parameters = Parameters::new();
    for (key, val) in params_val.as_object().unwrap() {
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
        bare_item if bare_item.is_i64() => {
            Ok(BareItem::Number(Num::Integer(bare_item.as_i64().unwrap())))
        }
        bare_item if bare_item.is_f64() => {
            Ok(Decimal::from_str(&serde_json::to_string(bare_item)?)
                .unwrap()
                .into())
        }
        bare_item if bare_item.is_boolean() => Ok(BareItem::Boolean(bare_item.as_bool().unwrap())),
        bare_item if bare_item.is_string() => Ok(BareItem::String(
            bare_item.as_str().unwrap().clone().to_owned(),
        )),
        bare_item if (bare_item.is_object() && bare_item["__type"] == "token") => Ok(
            BareItem::Token(bare_item["value"].as_str().unwrap().clone().to_owned()),
        ),
        bare_item if (bare_item.is_object() && bare_item["__type"] == "binary") => {
            let str_val = bare_item["value"].as_str().unwrap().clone();
            Ok(BareItem::ByteSeq(BASE32.decode(str_val.as_bytes())?))
        }
        _ => return Err("Unknown bare_item value".into()),
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
