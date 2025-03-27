use serde::Deserialize;
use serde_json::Value;
use sfv::{
    BareItem, Date, Dictionary, InnerList, Item, KeyRef, List, ListEntry, Parameters, Parser,
    SerializeValue, StringRef, TokenRef,
};
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
    fn serialize(&self) -> Result<String, sfv::Error> {
        match self {
            FieldType::Item(value) => value.serialize_value(),
            FieldType::List(value) => value.serialize_value(),
            FieldType::Dict(value) => value.serialize_value(),
        }
    }
}

fn run_test_case(test_case: &TestData) -> Result<(), Box<dyn Error>> {
    println!("- {}", test_case.name);

    let input = test_case
        .raw
        .as_ref()
        .ok_or("run_test_case: raw value is not specified")?
        .join(", ");

    let parser = Parser::new(&input);

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
    let actual_result = actual_field_value.serialize();
    if let Some(canonical_val) = &test_case.canonical {
        if canonical_val.is_empty() {
            assert!(actual_result.is_err());
        } else {
            assert_eq!(canonical_val[0], actual_result?);
        }
    } else {
        // If the canonical field is omitted, the canonical form is the input.
        assert_eq!(input, actual_result?);
    }

    Ok(())
}

fn run_test_case_serialization_only(test_case: &TestData) -> Result<(), Box<dyn Error>> {
    let expected_field_value = match build_expected_field_value(test_case) {
        Ok(v) => v,
        Err(_) => {
            assert!(test_case.must_fail);
            return Ok(());
        }
    };
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
    Ok(match test_case.header_type {
        HeaderType::Item => FieldType::Item(build_item(expected_value)?),
        HeaderType::List => FieldType::List(build_list(expected_value)?),
        HeaderType::Dictionary => FieldType::Dict(build_dict(expected_value)?),
    })
}

fn build_list_or_item(member: &Value) -> Result<ListEntry, Box<dyn Error>> {
    let member_as_array = member
        .as_array()
        .ok_or("build_list_or_item: list_or_item value is not an array")?;

    // If member is an array of arrays, then it represents InnerList, otherwise it's an Item
    Ok(if member_as_array[0].is_array() {
        build_inner_list(member)?.into()
    } else {
        build_item(member)?.into()
    })
}

fn build_dict(expected_value: &Value) -> Result<Dictionary, Box<dyn Error>> {
    let expected_array = expected_value
        .as_array()
        .ok_or("build_dict: expected value is not an array")?;

    let mut dict = Dictionary::new();

    for member in expected_array {
        let member = member
            .as_array()
            .ok_or("build_dict: expected dict member is not an array")?;
        let member_name = KeyRef::from_str(
            member[0]
                .as_str()
                .ok_or("build_dict: expected dict member name is not a str")?,
        )?;
        let member_value = &member[1];
        let item_or_inner_list: ListEntry = build_list_or_item(member_value)?;
        dict.insert(member_name.to_owned(), item_or_inner_list);
    }
    Ok(dict)
}

fn build_list(expected_value: &Value) -> Result<List, Box<dyn Error>> {
    expected_value
        .as_array()
        .ok_or("build_list: expected value is not an array")?
        .iter()
        .map(build_list_or_item)
        .collect()
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

fn build_bare_item(value: &Value) -> Result<BareItem, Box<dyn Error>> {
    Ok(match value {
        value if value.is_i64() => value.as_i64().unwrap().try_into()?,
        value if value.is_f64() => value.as_f64().unwrap().try_into()?,
        value if value.is_boolean() => value.as_bool().unwrap().into(),
        value if value.is_string() => StringRef::from_str(value.as_str().unwrap())?.into(),
        value if (value.is_object() && value["__type"] == "token") => TokenRef::from_str(
            value["value"]
                .as_str()
                .ok_or("build_bare_item: bare_item value is not a str")?,
        )?
        .into(),
        value if (value.is_object() && value["__type"] == "binary") => {
            let value = value["value"]
                .as_str()
                .ok_or("build_bare_item: bare_item value is not a str")?;

            base32::decode(base32::Alphabet::Rfc4648 { padding: true }, value)
                .ok_or("build_bare_item: invalid base32")?
                .into()
        }
        value if (value.is_object() && value["__type"] == "date") => Date::from_unix_seconds(
            value["value"]
                .as_i64()
                .ok_or("build_bare_item: bare_item value is not an i64")?
                .try_into()?,
        )
        .into(),
        value if (value.is_object() && value["__type"] == "displaystring") => {
            BareItem::DisplayString(
                value["value"]
                    .as_str()
                    .ok_or("build_bare_item: bare_item value is not a str")?
                    .to_owned(),
            )
        }
        _ => Err("build_bare_item: unknown bare_item value")?,
    })
}

fn build_parameters(params_value: &Value) -> Result<Parameters, Box<dyn Error>> {
    let mut parameters = Parameters::new();

    let parameters_array = params_value
        .as_array()
        .ok_or("build_parameters: params value is not an array")?;

    for member in parameters_array {
        let member = member
            .as_array()
            .ok_or("build_parameters: expected parameter is not an array")?;
        let key = KeyRef::from_str(
            member[0]
                .as_str()
                .ok_or("build_parameters: expected parameter name is not a str")?,
        )?;
        let value = &member[1];
        let itm = build_bare_item(value)?;
        parameters.insert(key.to_owned(), itm);
    }
    Ok(parameters)
}

fn run_tests(dir: PathBuf, is_serialization: bool) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;

        if entry.path().extension().unwrap_or_default() != "json" {
            continue;
        }

        println!("\n## Test suite file: {:?}\n", entry.file_name());

        let test_cases: Vec<TestData> = serde_json::from_reader(fs::File::open(entry.path())?)?;

        for test_data in &test_cases {
            if is_serialization {
                run_test_case_serialization_only(test_data)?;
            } else {
                run_test_case(test_data)?;
            }
        }
    }
    Ok(())
}

#[test]
fn run_spec_parse_serialize_tests() -> Result<(), Box<dyn Error>> {
    run_tests(
        env::current_dir()?.join("tests").join("spec_tests"),
        /*is_serialization=*/ false,
    )
}

#[test]
fn run_spec_serialize_only_tests() -> Result<(), Box<dyn Error>> {
    run_tests(
        env::current_dir()?
            .join("tests")
            .join("spec_tests")
            .join("serialisation-tests"),
        /*is_serialization=*/ true,
    )
}
