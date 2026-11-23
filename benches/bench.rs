use std::{borrow::BorrowMut, hint::black_box};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use sfv::{
    Decimal, DictSerializer, Dictionary, FieldType as _, Item, ItemSerializer, List,
    ListSerializer, Parser, TokenRef, integer, key_ref, string_ref, token_ref, visitor::Ignored,
};

criterion_main!(parsing, serializing);

criterion_group!(
    parsing,
    parsing_item,
    parsing_list,
    parsing_dict,
    parsing_string,
    parsing_display_string,
);

const ITEM_FIXTURE: &str =
    "c29tZXZlcnlsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXNhbnNvbWVvdGhlcmxvbmdsaW5l";
const ITEM_TOKEN: &TokenRef = token_ref(ITEM_FIXTURE);

const LIST_FIXTURE: &str = r#"a, abcdefghigklmnoprst, 123456785686457, 99999999999.999, (), ("somelongstringvalue" "anotherlongstringvalue";key=:c29tZXZlciBsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXM: 145)"#;

const DICT_FIXTURE: &str = r#"a, dict_key2=abcdefghigklmnoprst, dict_key3=123456785686457, dict_key4=("inner-list-member" :aW5uZXItbGlzdC1tZW1iZXI=:);key=aW5uZXItbGlzdC1wYXJhbWV0ZXJz"#;

fn parsing_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing_string");

    let unescaped =
        r#""c29tZXZlcnlsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXNhbnNvbWVvdGhlcmxvbmdsaW5l""#;
    let escaped =
        r#""c29tZXZlYy\\\"b25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXNhbnNvbWVvdGhlcmxvbmdsaW5l""#;

    for (kind, input) in [("unescaped", unescaped), ("escaped", escaped)] {
        group.bench_with_input(BenchmarkId::new("parsed", kind), input, |b, input| {
            b.iter(|| Parser::new(input).parse::<Item>().unwrap());
        });
        group.bench_with_input(BenchmarkId::new("visitor", kind), input, |b, input| {
            b.iter(|| Parser::new(input).parse_item_with_visitor(Ignored).unwrap());
        });
    }

    group.finish();
}

fn parsing_display_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing_display_string");

    let unescaped =
        r#"%"c29tZXZlcnlsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXNhbnNvbWVvdGhlcmxvbmdsaW5l""#;
    let escaped = r#"%"c29tZXZlYy%e2%82%ac29uZ3N0cmluZ3ZhbHVlcmVwcmVzZW50ZWRhc2J5dGVzYW5zb21lb3RoZXJsb25nbGluZSI""#;

    for (kind, input) in [("unescaped", unescaped), ("escaped", escaped)] {
        group.bench_with_input(BenchmarkId::new("parsed", kind), input, |b, input| {
            b.iter(|| Parser::new(input).parse::<Item>().unwrap());
        });
        group.bench_with_input(BenchmarkId::new("visitor", kind), input, |b, input| {
            b.iter(|| Parser::new(input).parse_item_with_visitor(Ignored).unwrap());
        });
    }

    group.finish();
}

fn parsing_item(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing_item");
    group.bench_with_input("parsed", ITEM_FIXTURE, |b, input| {
        b.iter(|| Parser::new(input).parse::<Item>().unwrap());
    });
    group.bench_with_input("visitor", ITEM_FIXTURE, |b, input| {
        b.iter(|| Parser::new(input).parse_item_with_visitor(Ignored).unwrap());
    });
    group.finish();
}

fn parsing_list(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing_list");
    group.bench_with_input("parsed", LIST_FIXTURE, |b, input| {
        b.iter(|| Parser::new(input).parse::<List>().unwrap());
    });
    group.bench_with_input("visitor", LIST_FIXTURE, |b, input| {
        b.iter(|| Parser::new(input).parse_list_with_visitor(Ignored).unwrap());
    });
    group.finish();
}

fn parsing_dict(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing_dict");
    group.bench_with_input("parsed", DICT_FIXTURE, |b, input| {
        b.iter(|| Parser::new(input).parse::<Dictionary>().unwrap());
    });
    group.bench_with_input("visitor", DICT_FIXTURE, |b, input| {
        b.iter(|| {
            Parser::new(input)
                .parse_dictionary_with_visitor(Ignored)
                .unwrap();
        });
    });
    group.finish();
}

criterion_group!(
    serializing,
    serializing_item,
    serializing_list,
    serializing_dict,
);

fn serialize_item_ref<W: BorrowMut<String>>(ser: ItemSerializer<W>) -> W {
    ser.bare_item(ITEM_TOKEN).finish()
}

fn serializing_item(c: &mut Criterion) {
    let parsed_item: Item = Parser::new(ITEM_FIXTURE).parse().unwrap();
    assert_eq!(
        parsed_item.serialize(),
        serialize_item_ref(ItemSerializer::new()),
    );

    let mut group = c.benchmark_group("serializing_item");
    group.bench_function("parsed", |b| {
        b.iter(|| parsed_item.serialize());
    });
    group.bench_function("ref", |b| {
        b.iter(|| serialize_item_ref(ItemSerializer::new()));
    });
    group.bench_function("ref_with_buffer", |b| {
        let mut buf = String::new();
        b.iter(|| {
            buf.clear();
            black_box(serialize_item_ref(ItemSerializer::with_buffer(&mut buf)));
        });
    });
    group.finish();
}

fn serialize_list_ref<W: BorrowMut<String>>(mut ser: ListSerializer<W>) -> Option<W> {
    ser.bare_item(const { token_ref("a") });
    ser.bare_item(const { token_ref("abcdefghigklmnoprst") });
    ser.bare_item(const { integer(123_456_785_686_457) });
    ser.bare_item(const { Decimal::from_integer_scaled_1000(integer(99_999_999_999_999)) });
    _ = ser.inner_list();
    {
        let mut ser = ser.inner_list();
        ser.bare_item(const { string_ref("somelongstringvalue") });
        ser.bare_item(const { string_ref("anotherlongstringvalue") })
            .parameter(
                const { key_ref("key") },
                b"somever longstringvaluerepresentedasbytes",
            );
        ser.bare_item(145);
    }
    ser.finish()
}

fn serializing_list(c: &mut Criterion) {
    let parsed_list: List = Parser::new(LIST_FIXTURE).parse().unwrap();
    assert_eq!(
        parsed_list.serialize(),
        serialize_list_ref(ListSerializer::new()),
    );

    let mut group = c.benchmark_group("serializing_list");
    group.bench_function("parsed", |b| {
        b.iter(|| parsed_list.serialize().unwrap());
    });
    group.bench_function("ref", |b| {
        b.iter(|| serialize_list_ref(ListSerializer::new()).unwrap());
    });
    group.bench_function("ref_with_buffer", |b| {
        let mut buf = String::new();
        b.iter(|| {
            buf.clear();
            black_box(serialize_list_ref(ListSerializer::with_buffer(&mut buf)).unwrap());
        });
    });
    group.finish();
}

fn serialize_dict_ref<W: BorrowMut<String>>(mut ser: DictSerializer<W>) -> Option<W> {
    ser.bare_item(const { key_ref("a") }, true);
    ser.bare_item(
        const { key_ref("dict_key2") },
        const { token_ref("abcdefghigklmnoprst") },
    );
    ser.bare_item(
        const { key_ref("dict_key3") },
        const { integer(123_456_785_686_457) },
    );
    {
        let mut ser = ser.inner_list(const { key_ref("dict_key4") });
        ser.bare_item(const { string_ref("inner-list-member") });
        ser.bare_item(b"inner-list-member");
        ser.finish().parameter(
            const { key_ref("key") },
            const { token_ref("aW5uZXItbGlzdC1wYXJhbWV0ZXJz") },
        );
    }
    ser.finish()
}

fn serializing_dict(c: &mut Criterion) {
    let parsed_dict: Dictionary = Parser::new(DICT_FIXTURE).parse().unwrap();
    assert_eq!(
        parsed_dict.serialize(),
        serialize_dict_ref(DictSerializer::new()),
    );

    let mut group = c.benchmark_group("serializing_dict");
    group.bench_function("parsed", |b| {
        b.iter(|| parsed_dict.serialize().unwrap());
    });
    group.bench_function("ref", |b| {
        b.iter(|| serialize_dict_ref(DictSerializer::new()).unwrap());
    });
    group.bench_function("ref_with_buffer", |b| {
        let mut buf = String::new();
        b.iter(|| {
            buf.clear();
            black_box(serialize_dict_ref(DictSerializer::with_buffer(&mut buf)).unwrap());
        });
    });
    group.finish();
}
