use serde::Deserialize;
use sfv::{
    BareItem, Date, Dictionary, InnerList, Item, Key, List, ListEntry, Parameters, Parser,
    SerializeValue,
};
use std::error::Error;
use std::path::Path;
use std::{env, fmt, fs, io};

#[derive(Debug, Deserialize)]
struct TestData {
    name: String,
    #[serde(flatten)]
    header_type: ExpectedHeaderType,
    #[serde(default)]
    must_fail: bool,
    canonical: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ParseTestData {
    #[serde(flatten)]
    data: TestData,
    raw: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", tag = "header_type", content = "expected")]
// https://github.com/httpwg/structured-field-tests/blob/main/README.md#test-format
enum ExpectedHeaderType {
    Item(Option<ExpectedItem>),
    List(Option<ExpectedList>),
    Dictionary(Option<ExpectedDict>),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__type", content = "value")]
enum ExpectedBareItem {
    #[serde(rename = "binary")]
    ByteSequence(String),
    #[serde(rename = "token")]
    Token(String),
    #[serde(rename = "date")]
    Date(i64),
    #[serde(rename = "displaystring")]
    DisplayString(String),
    #[serde(untagged)]
    Boolean(bool),
    #[serde(untagged)]
    Integer(i64),
    #[serde(untagged)]
    Decimal(f64),
    #[serde(untagged)]
    String(String),
}

type ExpectedParameters = Vec<(String, ExpectedBareItem)>;

type ExpectedItem = (ExpectedBareItem, ExpectedParameters);

type ExpectedInnerList = (Vec<ExpectedItem>, ExpectedParameters);

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ExpectedListEntry {
    Item(ExpectedItem),
    InnerList(ExpectedInnerList),
}

type ExpectedList = Vec<ExpectedListEntry>;

type ExpectedDict = Vec<(String, ExpectedListEntry)>;

trait TestCase: for<'a> Deserialize<'a> {
    fn run(self);
}

impl TestCase for ParseTestData {
    fn run(mut self) {
        fn check<T: PartialEq + fmt::Debug + SerializeValue>(
            test_case: &ParseTestData,
            parse: impl for<'a> FnOnce(Parser<'a>) -> Result<T, sfv::Error>,
            expected: Option<impl Build<T>>,
        ) {
            println!("- {}", test_case.data.name);
            let input = test_case.raw.join(", ");

            match parse(Parser::new(&input)) {
                Ok(actual) => {
                    assert!(!test_case.data.must_fail);
                    assert_eq!(
                        actual,
                        expected
                            .expect("expected value should be present")
                            .build()
                            .expect("build should succeed")
                    );

                    let serialized = actual.serialize_value();

                    match test_case.data.canonical {
                        // If the canonical field is omitted, the canonical form is the input.
                        None => {
                            assert_eq!(serialized.expect("serialization should succeed"), input)
                        }
                        Some(ref canonical) => {
                            // If the canonical field is an empty list, the serialization
                            // should be omitted, which corresponds to an error from `serialize_value`.
                            if canonical.is_empty() {
                                assert!(serialized.is_err());
                            } else {
                                assert_eq!(
                                    serialized.expect("serialization should succeed"),
                                    canonical[0]
                                );
                            }
                        }
                    }
                }
                Err(_) => assert!(test_case.data.must_fail),
            }
        }

        match self.data.header_type {
            ExpectedHeaderType::Item(ref mut expected) => {
                let expected = expected.take();
                check(&self, |p| p.parse_item(), expected);
            }
            ExpectedHeaderType::List(ref mut expected) => {
                let expected = expected.take();
                check(&self, |p| p.parse_list(), expected);
            }
            ExpectedHeaderType::Dictionary(ref mut expected) => {
                let expected = expected.take();
                check(&self, |p| p.parse_dictionary(), expected);
            }
        }
    }
}

impl TestCase for TestData {
    fn run(mut self) {
        fn check<T: SerializeValue>(test_case: &TestData, value: Option<impl Build<T>>) {
            println!("- {}", test_case.name);
            match value.expect("expected value should be present").build() {
                Ok(value) => match value.serialize_value() {
                    Ok(serialized) => {
                        assert!(!test_case.must_fail);
                        assert_eq!(
                            serialized,
                            test_case
                                .canonical
                                .as_ref()
                                .expect("canonical serialization should be present")[0]
                        )
                    }
                    Err(_) => assert!(test_case.must_fail),
                },
                Err(_) => assert!(test_case.must_fail),
            }
        }

        match self.header_type {
            ExpectedHeaderType::Item(ref mut expected) => {
                let expected = expected.take();
                check(&self, expected)
            }
            ExpectedHeaderType::List(ref mut expected) => {
                let expected = expected.take();
                check(&self, expected)
            }
            ExpectedHeaderType::Dictionary(ref mut expected) => {
                let expected = expected.take();
                check(&self, expected)
            }
        }
    }
}

trait Build<O> {
    fn build(self) -> Result<O, Box<dyn Error>>;
}

impl Build<ListEntry> for ExpectedListEntry {
    fn build(self) -> Result<ListEntry, Box<dyn Error>> {
        match self {
            Self::Item(value) => value.build().map(ListEntry::from),
            Self::InnerList(value) => value.build().map(ListEntry::from),
        }
    }
}

impl Build<Dictionary> for ExpectedDict {
    fn build(self) -> Result<Dictionary, Box<dyn Error>> {
        let mut dict = Dictionary::new();
        for (key, value) in self {
            let key = Key::try_from(key)?;
            let value = value.build()?;
            dict.insert(key, value);
        }
        Ok(dict)
    }
}

impl Build<List> for ExpectedList {
    fn build(self) -> Result<List, Box<dyn Error>> {
        self.into_iter().map(Build::build).collect()
    }
}

impl Build<InnerList> for ExpectedInnerList {
    fn build(self) -> Result<InnerList, Box<dyn Error>> {
        Ok(InnerList {
            items: self
                .0
                .into_iter()
                .map(Build::build)
                .collect::<Result<Vec<Item>, Box<dyn Error>>>()?,
            params: self.1.build()?,
        })
    }
}

impl Build<Item> for ExpectedItem {
    fn build(self) -> Result<Item, Box<dyn Error>> {
        Ok(Item {
            bare_item: self.0.build()?,
            params: self.1.build()?,
        })
    }
}

impl Build<BareItem> for ExpectedBareItem {
    fn build(self) -> Result<BareItem, Box<dyn Error>> {
        Ok(match self {
            Self::Integer(value) => value.try_into()?,
            Self::Decimal(value) => value.try_into()?,
            Self::Boolean(value) => value.into(),
            Self::String(value) => BareItem::String(value.try_into()?),
            Self::Token(value) => BareItem::Token(value.try_into()?),
            Self::ByteSequence(ref value) => {
                base32::decode(base32::Alphabet::Rfc4648 { padding: true }, value)
                    .ok_or("invalid base32")?
                    .into()
            }
            Self::Date(value) => Date::from_unix_seconds(value.try_into()?).into(),
            Self::DisplayString(value) => BareItem::DisplayString(value),
        })
    }
}

impl Build<Parameters> for ExpectedParameters {
    fn build(self) -> Result<Parameters, Box<dyn Error>> {
        let mut parameters = Parameters::new();
        for (key, value) in self {
            let key = Key::try_from(key)?;
            let value = value.build()?;
            parameters.insert(key, value);
        }
        Ok(parameters)
    }
}

fn run_tests<T: TestCase>(dir_path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(env::current_dir()?.join(dir_path))? {
        let entry = entry?;

        if entry.path().extension().unwrap_or_default() != "json" {
            continue;
        }

        println!("\n## Test suite file: {:?}\n", entry.file_name());

        let test_cases: Vec<T> =
            serde_json::from_reader(io::BufReader::new(fs::File::open(entry.path())?))?;

        for test_case in test_cases {
            test_case.run();
        }
    }
    Ok(())
}

#[test]
fn run_spec_parse_serialize_tests() -> Result<(), Box<dyn Error>> {
    run_tests::<ParseTestData>("tests/spec_tests")
}

#[test]
fn run_spec_serialize_only_tests() -> Result<(), Box<dyn Error>> {
    run_tests::<TestData>("tests/spec_tests/serialisation-tests")
}
