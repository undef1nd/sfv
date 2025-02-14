#[macro_use]
extern crate criterion;

use criterion::{BenchmarkId, Criterion};
use rust_decimal::prelude::FromPrimitive;
use sfv::{
    integer, string_ref, token_ref, Decimal, Parser, RefDictSerializer, RefItemSerializer,
    RefListSerializer, SerializeValue,
};

criterion_main!(parsing, serializing, ref_serializing);

criterion_group!(parsing, parsing_item, parsing_list, parsing_dict);

fn parsing_item(c: &mut Criterion) {
    let fixture =
        "c29tZXZlcnlsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXNhbnNvbWVvdGhlcmxvbmdsaW5l";
    c.bench_with_input(
        BenchmarkId::new("parsing_item", fixture),
        &fixture,
        move |bench, &input| {
            bench.iter(|| Parser::from_str(input).parse_item().unwrap());
        },
    );
}

fn parsing_list(c: &mut Criterion) {
    let fixture = r#"a, abcdefghigklmnoprst, 123456785686457, 99999999999.999, (), ("somelongstringvalue" "anotherlongstringvalue";key=:c29tZXZlciBsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXM: 145)"#;
    c.bench_with_input(
        BenchmarkId::new("parsing_list", fixture),
        &fixture,
        move |bench, &input| {
            bench.iter(|| Parser::from_str(input).parse_list().unwrap());
        },
    );
}

fn parsing_dict(c: &mut Criterion) {
    let fixture = r#"a, dict_key2=abcdefghigklmnoprst, dict_key3=123456785686457, dict_key4=("inner-list-member" :aW5uZXItbGlzdC1tZW1iZXI=:);key=aW5uZXItbGlzdC1wYXJhbWV0ZXJz"#;
    c.bench_with_input(
        BenchmarkId::new("parsing_dict", fixture),
        &fixture,
        move |bench, &input| {
            bench.iter(|| Parser::from_str(input).parse_dictionary().unwrap());
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
            let parsed_item = Parser::from_str(input).parse_item().unwrap();
            bench.iter(|| parsed_item.serialize_value().unwrap());
        },
    );
}

fn serializing_list(c: &mut Criterion) {
    let fixture = r#"a, abcdefghigklmnoprst, 123456785686457, 99999999999.999, (), ("somelongstringvalue" "anotherlongstringvalue";key=:c29tZXZlciBsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXM: 145)"#;
    c.bench_with_input(
        BenchmarkId::new("serializing_list", fixture),
        &fixture,
        move |bench, &input| {
            let parsed_list = Parser::from_str(input).parse_list().unwrap();
            bench.iter(|| parsed_list.serialize_value().unwrap());
        },
    );
}

fn serializing_dict(c: &mut Criterion) {
    let fixture = r#"a, dict_key2=abcdefghigklmnoprst, dict_key3=123456785686457, dict_key4=("inner-list-member" :aW5uZXItbGlzdC1tZW1iZXI=:);key=aW5uZXItbGlzdC1wYXJhbWV0ZXJz"#;
    c.bench_with_input(
        BenchmarkId::new("serializing_dict", fixture),
        &fixture,
        move |bench, &input| {
            let parsed_dict = Parser::from_str(input).parse_dictionary().unwrap();
            bench.iter(|| parsed_dict.serialize_value().unwrap());
        },
    );
}

criterion_group!(
    ref_serializing,
    serializing_ref_item,
    serializing_ref_list,
    serializing_ref_dict
);

fn serializing_ref_item(c: &mut Criterion) {
    let fixture =
        "c29tZXZlcnlsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXNhbnNvbWVvdGhlcmxvbmdsaW5l";
    c.bench_with_input(
        BenchmarkId::new("serializing_ref_item", fixture),
        &fixture,
        move |bench, &input| {
            bench.iter(|| {
                let ser = RefItemSerializer::new();
                ser.bare_item(input.as_bytes()).unwrap().finish()
            });
        },
    );
}

fn serializing_ref_list(c: &mut Criterion) {
    c.bench_function("serializing_ref_list", move |bench| {
        bench.iter(|| {
            let ser = RefListSerializer::new();
            ser.bare_item(token_ref("a"))
                .unwrap()
                .bare_item(token_ref("abcdefghigklmnoprst"))
                .unwrap()
                .bare_item(integer(123456785686457))
                .unwrap()
                .bare_item(Decimal::from_f64(99999999999.999).unwrap())
                .unwrap()
                .open_inner_list()
                .close_inner_list()
                .open_inner_list()
                .inner_list_bare_item(string_ref("somelongstringvalue"))
                .unwrap()
                .inner_list_bare_item(string_ref("anotherlongstringvalue"))
                .unwrap()
                .inner_list_parameter(
                    "key",
                    "somever longstringvaluerepresentedasbytes".as_bytes(),
                )
                .unwrap()
                .inner_list_bare_item(145)
                .unwrap()
                .close_inner_list()
                .finish()
        });
    });
}

fn serializing_ref_dict(c: &mut Criterion) {
    c.bench_function("serializing_ref_dict", move |bench| {
        bench.iter(|| {
            RefDictSerializer::new()
                .bare_item_member("a", true)
                .unwrap()
                .bare_item_member("dict_key2", token_ref("abcdefghigklmnoprst"))
                .unwrap()
                .bare_item_member("dict_key3", integer(123456785686457))
                .unwrap()
                .open_inner_list("dict_key4")
                .unwrap()
                .inner_list_bare_item(string_ref("inner-list-member"))
                .unwrap()
                .inner_list_bare_item("inner-list-member".as_bytes())
                .unwrap()
                .close_inner_list()
                .parameter("key", token_ref("aW5uZXItbGlzdC1wYXJhbWV0ZXJz"))
                .unwrap()
                .finish()
        });
    });
}
