use std::{env, error::Error, fmt, fs, io, path::Path};

use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeOwned},
};
use sfv::{
    BareItem, Date, Dictionary, FieldType, InnerList, Item, Key, List, ListEntry, Parameters,
    Parser,
};

#[derive(Deserialize)]
#[serde(bound = "")]
struct Test<C: TestCase> {
    name: String,
    #[serde(flatten)]
    header_type: ExpectedHeaderType<C>,
    #[serde(flatten)]
    case: C,
}

#[derive(Debug, Deserialize)]
struct TestData {
    #[serde(default)]
    must_fail: bool,
    #[serde(default)]
    can_fail: bool,
    canonical: Option<[String; 1]>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Canonical {
    Empty([String; 0]),
    Single([String; 1]),
}

#[derive(Debug, Deserialize)]
struct ParseTestData {
    #[serde(default)]
    must_fail: bool,
    #[serde(default)]
    can_fail: bool,
    canonical: Option<Canonical>,
    raw: Vec<String>,
}

#[derive(Deserialize)]
#[serde(
    rename_all = "lowercase",
    tag = "header_type",
    content = "expected",
    bound = ""
)]
// https://github.com/httpwg/structured-field-tests/blob/main/README.md#test-format
enum ExpectedHeaderType<C: TestCase> {
    Item(C::Expected<ExpectedItem>),
    List(C::Expected<ExpectedList>),
    Dictionary(C::Expected<ExpectedDict>),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "__type", content = "value")]
enum ExpectedBareItem {
    #[serde(rename = "binary", deserialize_with = "deserialize_base32")]
    ByteSequence(Vec<u8>),
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

fn deserialize_base32<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    let str = String::deserialize(deserializer)?;
    base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &str)
        .ok_or_else(|| de::Error::custom("invalid base32"))
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

trait TestCase: DeserializeOwned {
    type Expected<T: DeserializeOwned>: DeserializeOwned;

    fn check<T: PartialEq + fmt::Debug + FieldType>(self, expected: Self::Expected<impl Build<T>>);
}

impl TestCase for ParseTestData {
    type Expected<T: DeserializeOwned> = Option<T>;

    fn check<T: PartialEq + fmt::Debug + FieldType>(self, expected: Option<impl Build<T>>) {
        let input = self.raw.join(", ");

        match Parser::new(&input).parse::<T>() {
            Ok(actual) => {
                assert!(!self.must_fail);
                assert_eq!(
                    actual,
                    expected
                        .expect("expected value should be present")
                        .build()
                        .expect("build should succeed")
                );

                // If the canonical field is omitted, the canonical form is the input.
                // If the canonical field is an empty list, the serialization should be
                // omitted, which corresponds to `None` from `serialize`.
                assert_eq!(
                    actual.serialize().into(),
                    match self.canonical {
                        None => Some(input),
                        Some(Canonical::Empty([])) => None,
                        Some(Canonical::Single([canonical])) => Some(canonical),
                    },
                );
            }
            Err(_) => assert!(self.must_fail || self.can_fail),
        }
    }
}

impl TestCase for TestData {
    type Expected<T: DeserializeOwned> = T;

    fn check<T: PartialEq + fmt::Debug + FieldType>(self, expected: impl Build<T>) {
        match expected.build() {
            Ok(value) => match value.serialize().into() {
                Some(serialized) => {
                    assert!(!self.must_fail);
                    let [canonical] = self
                        .canonical
                        .expect("canonical serialization should be present");
                    assert_eq!(serialized, canonical);
                }
                None => assert!(self.must_fail || self.can_fail),
            },
            Err(_) => assert!(self.must_fail || self.can_fail),
        }
    }
}

trait Build<O>: DeserializeOwned {
    fn build(self) -> Result<O, sfv::Error>;
}

impl Build<ListEntry> for ExpectedListEntry {
    fn build(self) -> Result<ListEntry, sfv::Error> {
        match self {
            Self::Item(value) => value.build().map(ListEntry::from),
            Self::InnerList(value) => value.build().map(ListEntry::from),
        }
    }
}

impl Build<Dictionary> for ExpectedDict {
    fn build(self) -> Result<Dictionary, sfv::Error> {
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
    fn build(self) -> Result<List, sfv::Error> {
        self.into_iter().map(Build::build).collect()
    }
}

impl Build<InnerList> for ExpectedInnerList {
    fn build(self) -> Result<InnerList, sfv::Error> {
        Ok(InnerList {
            items: self
                .0
                .into_iter()
                .map(Build::build)
                .collect::<Result<Vec<Item>, sfv::Error>>()?,
            params: self.1.build()?,
        })
    }
}

impl Build<Item> for ExpectedItem {
    fn build(self) -> Result<Item, sfv::Error> {
        Ok(Item {
            bare_item: self.0.build()?,
            params: self.1.build()?,
        })
    }
}

impl Build<BareItem> for ExpectedBareItem {
    fn build(self) -> Result<BareItem, sfv::Error> {
        Ok(match self {
            Self::Integer(value) => value.try_into()?,
            Self::Decimal(value) => value.try_into()?,
            Self::Boolean(value) => value.into(),
            Self::String(value) => BareItem::String(value.try_into()?),
            Self::Token(value) => BareItem::Token(value.try_into()?),
            Self::ByteSequence(value) => value.into(),
            Self::Date(value) => Date::from_unix_seconds(value.try_into()?).into(),
            Self::DisplayString(value) => BareItem::DisplayString(value),
        })
    }
}

impl Build<Parameters> for ExpectedParameters {
    fn build(self) -> Result<Parameters, sfv::Error> {
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

        println!("\n## Test suite file: {}\n", entry.file_name().display());

        let tests: Vec<Test<T>> =
            serde_json::from_reader(io::BufReader::new(fs::File::open(entry.path())?))?;

        for test in tests {
            println!("- {}", test.name);
            match test.header_type {
                ExpectedHeaderType::Item(expected) => test.case.check(expected),
                ExpectedHeaderType::List(expected) => test.case.check(expected),
                ExpectedHeaderType::Dictionary(expected) => test.case.check(expected),
            }
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
