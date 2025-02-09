use serde::Deserialize;
use serde_json::Value;
use sfv::FromStr;
use sfv::Parser;
use sfv::SerializeValue;
use sfv::{BareItem, Decimal, Dictionary, InnerList, Item, List, ListEntry, Parameters};
use std::error::Error;
use std::path::PathBuf;
use std::{env, fs};

#[derive(Debug, Deserialize)]
struct TestData {
    name: String,
    raw: Option<Vec<String>>,
    header_type: HeaderType,
    expected: Option<Value>,
    #[serde(default)]
    must_fail: bool,
    canonical: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum HeaderType {
    Item,
    List,
    Dictionary,
}

#[derive(Debug, PartialEq)]
enum FieldType {
    Item(Item),
    List(List),
    Dict(Dictionary),
}
impl FieldType {
    fn serialize(&self) -> Result<String, &'static str> {
        match self {
            FieldType::Item(value) => value.serialize_value(),
            FieldType::List(value) => value.serialize_value(),
            FieldType::Dict(value) => value.serialize_value(),
        }
    }
}

fn run_test_case(test_case: &TestData) -> Result<(), Box<dyn Error>> {
    println!("- {}", &test_case.name);

    let input = test_case
        .raw
        .as_ref()
        .ok_or("run_test_case: raw value is not specified")?
        .join(", ");

    let parser = Parser::from_str(&input);

    let actual_result = match test_case.header_type {
        HeaderType::Item => parser.parse_item().map(FieldType::Item),
        HeaderType::List => parser.parse_list().map(FieldType::List),
        HeaderType::Dictionary => parser.parse_dictionary().map(FieldType::Dict),
    };

    // Check that actual result for must_fail tests is Err
    if test_case.must_fail {
        assert!(actual_result.is_err());
        return Ok(());
    }

    let expected_field_value = build_expected_field_value(test_case)?;
    let actual_field_value = actual_result?;

    // Test parsing
    assert_eq!(expected_field_value, actual_field_value);

    // Test serialization
    if let Some(canonical_val) = &test_case.canonical {
        let actual_result = actual_field_value.serialize();
        if canonical_val.is_empty() {
            assert!(actual_result.is_err());
        } else {
            assert_eq!(canonical_val[0], actual_result?);
        }
    }
    Ok(())
}

fn run_test_case_serialization_only(test_case: &TestData) -> Result<(), Box<dyn Error>> {
    let expected_field_value = build_expected_field_value(test_case)?;
    let actual_result = expected_field_value.serialize();

    if test_case.must_fail {
        assert!(actual_result.is_err());
        return Ok(());
    }

    // Test serialization
    if let Some(canonical_val) = &test_case.canonical {
        if canonical_val.is_empty() {
            assert!(actual_result.is_err());
        } else {
            assert_eq!(canonical_val[0], actual_result?);
        }
    }

    Ok(())
}

fn build_expected_field_value(test_case: &TestData) -> Result<FieldType, Box<dyn Error>> {
    let expected_value = test_case
        .expected
        .as_ref()
        .ok_or("build_expected_field_value: test's expected value is not specified")?;

    // Build expected Structured Field Value from serde Value
    match test_case.header_type {
        HeaderType::Item => {
            let item = build_item(expected_value)?;
            Ok(FieldType::Item(item))
        }
        HeaderType::List => {
            let list = build_list(expected_value)?;
            Ok(FieldType::List(list))
        }
        HeaderType::Dictionary => {
            let dict = build_dict(expected_value)?;
            Ok(FieldType::Dict(dict))
        }
    }
}

fn build_list_or_item(member: &Value) -> Result<ListEntry, Box<dyn Error>> {
    let member_as_array = member
        .as_array()
        .ok_or("build_list_or_item: list_or_item value is not an array")?;

    // If member is an array of arrays, then it represents InnerList, otherwise it's an Item
    let list_entry: ListEntry = if member_as_array[0].is_array() {
        build_inner_list(member)?.into()
    } else {
        build_item(member)?.into()
    };
    Ok(list_entry)
}

fn build_dict(expected_value: &Value) -> Result<Dictionary, Box<dyn Error>> {
    let expected_array = expected_value
        .as_array()
        .ok_or("build_dict: expected value is not an array")?;

    let mut dict = Dictionary::new();

    if expected_array.is_empty() {
        return Ok(dict);
    }

    for member in expected_array.iter() {
        let member = member
            .as_array()
            .ok_or("build_dict: expected dict member is not an array")?;
        let member_name = member[0]
            .as_str()
            .ok_or("build_dict: expected dict member name is not a str")?;
        let member_value = &member[1];
        let item_or_inner_list: ListEntry = build_list_or_item(member_value)?;
        dict.insert(member_name.to_owned(), item_or_inner_list);
    }
    Ok(dict)
}

fn build_list(expected_value: &Value) -> Result<List, Box<dyn Error>> {
    let expected_as_array = expected_value
        .as_array()
        .ok_or("build_list: expected value is not an array")?;

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
        .ok_or("build_inner_list: inner list is not an array")?;

    let inner_list_items = &inner_list_as_array[0];
    let inner_list_params = &inner_list_as_array[1];

    let mut items = vec![];
    for item_value in inner_list_items
        .as_array()
        .ok_or("build_inner_list: inner list items value is not an array")?
        .iter()
    {
        items.push(build_item(item_value)?);
    }

    let params = build_parameters(inner_list_params)?;

    Ok(InnerList { items, params })
}

fn build_item(expected_value: &Value) -> Result<Item, Box<dyn Error>> {
    // item = [bare_item, {params}]
    let expected_array = expected_value
        .as_array()
        .ok_or("build_item: expected value is not an array")?;

    // Item array must contain 2 members only
    if expected_array.len() != 2 {
        return Err("Not an item".into());
    }

    let bare_item_val = &expected_array[0];
    let params_val = &expected_array[1];
    let bare_item = build_bare_item(bare_item_val)?;
    let params = build_parameters(params_val)?;

    Ok(Item { bare_item, params })
}

fn build_bare_item(bare_item_value: &Value) -> Result<BareItem, Box<dyn Error>> {
    match bare_item_value {
        bare_item if bare_item.is_i64() => Ok(BareItem::Integer(
            bare_item
                .as_i64()
                .ok_or("build_bare_item: bare_item value is not an i64")?,
        )),
        bare_item if bare_item.is_f64() => {
            let decimal = Decimal::from_str(&serde_json::to_string(bare_item)?)?;
            Ok(BareItem::Decimal(decimal))
        }
        bare_item if bare_item.is_boolean() => Ok(BareItem::Boolean(
            bare_item
                .as_bool()
                .ok_or("build_bare_item: bare_item value is not a bool")?,
        )),
        bare_item if bare_item.is_string() => Ok(BareItem::String(
            bare_item
                .as_str()
                .ok_or("build_bare_item: bare_item value is not a str")?
                .to_owned(),
        )),
        bare_item if (bare_item.is_object() && bare_item["__type"] == "token") => {
            Ok(BareItem::Token(
                bare_item["value"]
                    .as_str()
                    .ok_or("build_bare_item: bare_item value is not a str")?
                    .to_owned(),
            ))
        }
        bare_item if (bare_item.is_object() && bare_item["__type"] == "binary") => {
            let str_val = bare_item["value"]
                .as_str()
                .ok_or("build_bare_item: bare_item value is not a str")?;
            Ok(BareItem::ByteSeq(
                base32::decode(base32::Alphabet::Rfc4648 { padding: true }, str_val)
                    .ok_or("build_bare_item: invalid base32")?,
            ))
        }
        _ => Err("build_bare_item: unknown bare_item value".into()),
    }
}

fn build_parameters(params_value: &Value) -> Result<Parameters, Box<dyn Error>> {
    let mut parameters = Parameters::new();

    let parameters_array = params_value
        .as_array()
        .ok_or("build_parameters: params value is not an array")?;
    if parameters_array.is_empty() {
        return Ok(parameters);
    };

    for member in parameters_array.iter() {
        let member = member
            .as_array()
            .ok_or("build_parameters: expected parameter is not an array")?;
        let key = member[0]
            .as_str()
            .ok_or("build_parameters: expected parameter name is not a str")?;
        let value = &member[1];
        let itm = build_bare_item(value)?;
        parameters.insert(key.to_owned(), itm);
    }
    Ok(parameters)
}

fn run_test_suite(tests_file: PathBuf, is_serialization: bool) -> Result<(), Box<dyn Error>> {
    let test_cases: Vec<TestData> = serde_json::from_reader(fs::File::open(tests_file)?)?;
    for test_data in test_cases.iter() {
        if is_serialization {
            run_test_case_serialization_only(test_data)?;
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
        .filter(|fp| {
            fp.path().extension().unwrap_or_default() == "json"
            // These are only supported in RFC 9651.
            && fp.path().file_stem().unwrap_or_default() != "date"
            && fp.path().file_stem().unwrap_or_default() != "display-string"
        });

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
    let read_dir = match fs::read_dir(test_suites_dir) {
        Ok(dir) => dir,
        _ => panic!("Test suite directory not found! Check that the spec_tests git submodule has been retrieved.")
    };

    let json_files = read_dir
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
