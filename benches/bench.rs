#[macro_use]
extern crate criterion;

use criterion::{BenchmarkId, Criterion};
use rust_decimal::prelude::FromPrimitive;
use sfv::{BareItem, Decimal, Parser, RefDictSerializer, SerializeValue};
use sfv::{RefBareItem, RefItemSerializer, RefListSerializer};

criterion_main!(parsing, serializing);

criterion_group!(parsing, parsing_item, parsing_list, parsing_dict);

fn parsing_item(c: &mut Criterion) {
    let fixture =
        "c29tZXZlcnlsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXNhbnNvbWVvdGhlcmxvbmdsaW5l";
    c.bench_with_input(
        BenchmarkId::new("parsing_item", fixture),
        &fixture,
        move |bench, &input| {
            bench.iter(|| Parser::parse_item(input.as_bytes()).unwrap());
        },
    );
}

fn parsing_list(c: &mut Criterion) {
    let fixture = "a, abcdefghigklmnoprst, 123456785686457, 99999999999.999, (), (\"somelongstringvalue\" \"anotherlongstringvalue\";key=:c29tZXZlciBsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXM: 145)";
    c.bench_with_input(
        BenchmarkId::new("parsing_list", fixture),
        &fixture,
        move |bench, &input| {
            bench.iter(|| Parser::parse_list(input.as_bytes()).unwrap());
        },
    );
}

fn parsing_dict(c: &mut Criterion) {
    let fixture = "a, dict_key2=abcdefghigklmnoprst, dict_key3=123456785686457, dict_key4=(\"inner-list-member\" :aW5uZXItbGlzdC1tZW1iZXI=:);key=aW5uZXItbGlzdC1wYXJhbWV0ZXJz";
    c.bench_with_input(
        BenchmarkId::new("parsing_dict", fixture),
        &fixture,
        move |bench, &input| {
            bench.iter(|| Parser::parse_dictionary(input.as_bytes()).unwrap());
        },
    );
}

criterion_group!(
    serializing,
    serializing_item,
    serializing_list,
    serializing_dict
);

fn serializing_item(c: &mut Criterion) {
    let fixture =
        "c29tZXZlcnlsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXNhbnNvbWVvdGhlcmxvbmdsaW5l";
    c.bench_with_input(
        BenchmarkId::new("serializing_item", fixture),
        &fixture,
        move |bench, &input| {
            let parsed_item = Parser::parse_item(input.as_bytes()).unwrap();
            bench.iter(|| parsed_item.serialize_value().unwrap());
        },
    );
}

fn serializing_list(c: &mut Criterion) {
    let fixture = "a, abcdefghigklmnoprst, 123456785686457, 99999999999.999, (), (\"somelongstringvalue\" \"anotherlongstringvalue\";key=:c29tZXZlciBsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXM: 145)";
    c.bench_with_input(
        BenchmarkId::new("serializing_list", fixture),
        &fixture,
        move |bench, &input| {
            let parsed_list = Parser::parse_list(input.as_bytes()).unwrap();
            bench.iter(|| parsed_list.serialize_value().unwrap());
        },
    );
}

fn serializing_dict(c: &mut Criterion) {
    let fixture = "a, dict_key2=abcdefghigklmnoprst, dict_key3=123456785686457, dict_key4=(\"inner-list-member\" :aW5uZXItbGlzdC1tZW1iZXI=:);key=aW5uZXItbGlzdC1wYXJhbWV0ZXJz";
    c.bench_with_input(
        BenchmarkId::new("serializing_dict", fixture),
        &fixture,
        move |bench, &input| {
            let parsed_dict = Parser::parse_dictionary(input.as_bytes()).unwrap();
            bench.iter(|| parsed_dict.serialize_value().unwrap());
        },
    );
}
