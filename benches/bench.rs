#[macro_use]
extern crate criterion;

use criterion::{BenchmarkId, Criterion};
use sfv::{
    integer, key_ref, string_ref, token_ref, Decimal, DictSerializer, Dictionary, FieldType, Item,
    ItemSerializer, List, ListSerializer, Parser,
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
            bench.iter(|| Parser::new(input).parse::<Item>().unwrap());
        },
    );
}

fn parsing_list(c: &mut Criterion) {
    let fixture = r#"a, abcdefghigklmnoprst, 123456785686457, 99999999999.999, (), ("somelongstringvalue" "anotherlongstringvalue";key=:c29tZXZlciBsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXM: 145)"#;
    c.bench_with_input(
        BenchmarkId::new("parsing_list", fixture),
        &fixture,
        move |bench, &input| {
            bench.iter(|| Parser::new(input).parse::<List>().unwrap());
        },
    );
}

fn parsing_dict(c: &mut Criterion) {
    let fixture = r#"a, dict_key2=abcdefghigklmnoprst, dict_key3=123456785686457, dict_key4=("inner-list-member" :aW5uZXItbGlzdC1tZW1iZXI=:);key=aW5uZXItbGlzdC1wYXJhbWV0ZXJz"#;
    c.bench_with_input(
        BenchmarkId::new("parsing_dict", fixture),
        &fixture,
        move |bench, &input| {
            bench.iter(|| Parser::new(input).parse::<Dictionary>().unwrap());
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
            let parsed_item: Item = Parser::new(input).parse().unwrap();
            bench.iter(|| parsed_item.serialize());
        },
    );
}

fn serializing_list(c: &mut Criterion) {
    let fixture = r#"a, abcdefghigklmnoprst, 123456785686457, 99999999999.999, (), ("somelongstringvalue" "anotherlongstringvalue";key=:c29tZXZlciBsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXM: 145)"#;
    c.bench_with_input(
        BenchmarkId::new("serializing_list", fixture),
        &fixture,
        move |bench, &input| {
            let parsed_list: List = Parser::new(input).parse().unwrap();
            bench.iter(|| parsed_list.serialize().unwrap());
        },
    );
}

fn serializing_dict(c: &mut Criterion) {
    let fixture = r#"a, dict_key2=abcdefghigklmnoprst, dict_key3=123456785686457, dict_key4=("inner-list-member" :aW5uZXItbGlzdC1tZW1iZXI=:);key=aW5uZXItbGlzdC1wYXJhbWV0ZXJz"#;
    c.bench_with_input(
        BenchmarkId::new("serializing_dict", fixture),
        &fixture,
        move |bench, &input| {
            let parsed_dict: Dictionary = Parser::new(input).parse().unwrap();
            bench.iter(|| parsed_dict.serialize().unwrap());
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
                let ser = ItemSerializer::new();
                ser.bare_item(input.as_bytes()).finish()
            });
        },
    );
}

fn serializing_ref_list(c: &mut Criterion) {
    c.bench_function("serializing_ref_list", move |bench| {
        bench.iter(|| {
            let mut ser = ListSerializer::new();
            _ = ser.bare_item(token_ref("a"));
            _ = ser.bare_item(token_ref("abcdefghigklmnoprst"));
            _ = ser.bare_item(integer(123_456_785_686_457));
            _ = ser.bare_item(Decimal::try_from(99_999_999_999.999).unwrap());
            _ = ser.inner_list();
            {
                let mut ser = ser.inner_list();
                _ = ser.bare_item(string_ref("somelongstringvalue"));
                _ = ser
                    .bare_item(string_ref("anotherlongstringvalue"))
                    .parameter(
                        key_ref("key"),
                        "somever longstringvaluerepresentedasbytes".as_bytes(),
                    );
                _ = ser.bare_item(145);
            }
            ser.finish()
        });
    });
}

fn serializing_ref_dict(c: &mut Criterion) {
    c.bench_function("serializing_ref_dict", move |bench| {
        bench.iter(|| {
            let mut ser = DictSerializer::new();
            _ = ser.bare_item(key_ref("a"), true);
            _ = ser.bare_item(key_ref("dict_key2"), token_ref("abcdefghigklmnoprst"));
            _ = ser.bare_item(key_ref("dict_key3"), integer(123_456_785_686_457));
            {
                let mut ser = ser.inner_list(key_ref("dict_key4"));
                _ = ser.bare_item(string_ref("inner-list-member"));
                _ = ser.bare_item("inner-list-member".as_bytes());
                _ = ser
                    .finish()
                    .parameter(key_ref("key"), token_ref("aW5uZXItbGlzdC1wYXJhbWV0ZXJz"));
            }
            ser.finish()
        });
    });
}
