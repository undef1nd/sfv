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
struct RawTest<C: TestCase> {
    name: String,
    #[serde(flatten)]
    header_type: HeaderType<
        C::RawExpected<ExpectedItem>,
        C::RawExpected<ExpectedList>,
        C::RawExpected<ExpectedDict>,
    >,
    #[serde(default)]
    must_fail: bool,
    #[serde(default)]
    can_fail: bool,
    canonical: Option<C::RawCanonical>,
    #[serde(flatten)]
    case: C,
}

#[derive(Deserialize)]
#[serde(try_from = "RawTest<C>", bound = "")]
struct Test<C: TestCase> {
    name: String,
    header_type: HeaderType<
        C::Validated<ExpectedItem>,
        C::Validated<ExpectedList>,
        C::Validated<ExpectedDict>,
    >,
    case: C,
}

impl<C: TestCase> TryFrom<RawTest<C>> for Test<C> {
    type Error = Box<dyn Error>;

    fn try_from(raw: RawTest<C>) -> Result<Self, Self::Error> {
        let expectation = if raw.must_fail {
            if raw.can_fail {
                return Err("must_fail and can_fail cannot both be true".into());
            }
            if raw.canonical.is_some() {
                return Err("canonical should be omitted when must_fail is true".into());
            }
            Expectation::MustFail
        } else {
            Expectation::Valid {
                expected: raw.canonical,
                can_fail: raw.can_fail,
            }
        };

        let header_type = match raw.header_type {
            HeaderType::Item(expected) => HeaderType::Item(C::validate(expectation, expected)?),
            HeaderType::List(expected) => HeaderType::List(C::validate(expectation, expected)?),
            HeaderType::Dictionary(expected) => {
                HeaderType::Dictionary(C::validate(expectation, expected)?)
            }
        };
        Ok(Self {
            name: raw.name,
            header_type,
            case: raw.case,
        })
    }
}

enum Expectation<T> {
    MustFail,
    Valid { expected: T, can_fail: bool },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Canonical {
    Empty([String; 0]),
    Single([String; 1]),
}

#[derive(Debug, Deserialize)]
struct ParseTestData {
    raw: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SerializeTestData {}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase", tag = "header_type", content = "expected")]
// https://github.com/httpwg/structured-field-tests/blob/main/README.md#test-format
enum HeaderType<I, L, D> {
    Item(I),
    List(L),
    Dictionary(D),
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
    type RawCanonical: DeserializeOwned;
    type RawExpected<E: Build>: DeserializeOwned;
    type Validated<E: Build>;

    fn validate<E: Build>(
        expectation: Expectation<Option<Self::RawCanonical>>,
        expected: Self::RawExpected<E>,
    ) -> Result<Self::Validated<E>, Box<dyn Error>>
    where
        E::Output: PartialEq + fmt::Debug + FieldType;

    fn check<E: Build>(self, validated: Self::Validated<E>)
    where
        E::Output: PartialEq + fmt::Debug + FieldType;
}

impl TestCase for ParseTestData {
    type RawCanonical = Canonical;
    type RawExpected<E: Build> = Option<E>;
    type Validated<E: Build> = Expectation<(E::Output, Option<Canonical>)>;

    fn validate<E: Build>(
        expectation: Expectation<Option<Canonical>>,
        expected: Option<E>,
    ) -> Result<Self::Validated<E>, Box<dyn Error>>
    where
        E::Output: PartialEq + fmt::Debug + FieldType,
    {
        match expectation {
            Expectation::MustFail => {
                if expected.is_some() {
                    return Err("expected should be omitted when must_fail is true".into());
                }
                Ok(Expectation::MustFail)
            }
            Expectation::Valid {
                expected: canonical,
                can_fail,
            } => {
                let expected = expected
                    .ok_or("expected value should be present")?
                    .build()?;
                Ok(Expectation::Valid {
                    expected: (expected, canonical),
                    can_fail,
                })
            }
        }
    }

    fn check<E: Build>(self, expectation: Self::Validated<E>)
    where
        E::Output: PartialEq + fmt::Debug + FieldType,
    {
        let input = self.raw.join(", ");
        let parsed = Parser::new(&input).parse::<E::Output>();

        match expectation {
            Expectation::MustFail => assert!(parsed.is_err()),
            Expectation::Valid {
                expected: (expected, canonical),
                can_fail,
            } => match parsed {
                Ok(actual) => {
                    assert_eq!(actual, expected);

                    // If the canonical field is omitted, the canonical form is the input.
                    // If the canonical field is an empty list, the serialization should be
                    // omitted, which corresponds to `None` from `serialize`.
                    assert_eq!(
                        actual.serialize().into(),
                        match canonical {
                            None => Some(input),
                            Some(Canonical::Empty([])) => None,
                            Some(Canonical::Single([canonical])) => Some(canonical),
                        },
                    );
                }
                Err(_) => assert!(can_fail),
            },
        }
    }
}

impl TestCase for SerializeTestData {
    type RawCanonical = [String; 1];
    type RawExpected<E: Build> = E;
    type Validated<E: Build> = (E, Expectation<String>);

    fn validate<E: Build>(
        expectation: Expectation<Option<[String; 1]>>,
        expected: E,
    ) -> Result<Self::Validated<E>, Box<dyn Error>>
    where
        E::Output: PartialEq + fmt::Debug + FieldType,
    {
        let expectation = match expectation {
            Expectation::MustFail => Expectation::MustFail,
            Expectation::Valid {
                expected: canonical,
                can_fail,
            } => {
                let [canonical] = canonical.ok_or("canonical serialization should be present")?;
                Expectation::Valid {
                    expected: canonical,
                    can_fail,
                }
            }
        };
        Ok((expected, expectation))
    }

    fn check<E: Build>(self, (expected, expectation): Self::Validated<E>)
    where
        E::Output: PartialEq + fmt::Debug + FieldType,
    {
        let serialized = expected.build().ok().and_then(|v| v.serialize().into());

        match expectation {
            Expectation::MustFail => assert!(serialized.is_none()),
            Expectation::Valid {
                expected: canonical,
                can_fail,
            } => match serialized {
                Some(serialized) => assert_eq!(serialized, canonical),
                None => assert!(can_fail),
            },
        }
    }
}

trait Build: DeserializeOwned {
    type Output;

    fn build(self) -> Result<Self::Output, sfv::Error>;
}

impl Build for ExpectedListEntry {
    type Output = ListEntry;

    fn build(self) -> Result<ListEntry, sfv::Error> {
        match self {
            Self::Item(value) => value.build().map(ListEntry::from),
            Self::InnerList(value) => value.build().map(ListEntry::from),
        }
    }
}

impl Build for ExpectedDict {
    type Output = Dictionary;

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

impl Build for ExpectedList {
    type Output = List;

    fn build(self) -> Result<List, sfv::Error> {
        self.into_iter().map(Build::build).collect()
    }
}

impl Build for ExpectedInnerList {
    type Output = InnerList;

    fn build(self) -> Result<InnerList, sfv::Error> {
        Ok(InnerList {
            items: self
                .0
                .into_iter()
                .map(Build::build)
                .collect::<Result<_, _>>()?,
            params: self.1.build()?,
        })
    }
}

impl Build for ExpectedItem {
    type Output = Item;

    fn build(self) -> Result<Item, sfv::Error> {
        Ok(Item {
            bare_item: self.0.build()?,
            params: self.1.build()?,
        })
    }
}

impl Build for ExpectedBareItem {
    type Output = BareItem;

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

impl Build for ExpectedParameters {
    type Output = Parameters;

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
                HeaderType::Item(validated) => test.case.check(validated),
                HeaderType::List(validated) => test.case.check(validated),
                HeaderType::Dictionary(validated) => test.case.check(validated),
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
    run_tests::<SerializeTestData>("tests/spec_tests/serialisation-tests")
}
