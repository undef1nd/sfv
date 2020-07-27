#[macro_use]
extern crate bencher;

use bencher::Bencher;
use sfv::{Parser, SerializeValue};

fn parsing_item(bench: &mut Bencher) {
    let input = "c29tZXZlcnlsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXNhbnNvbWVvdGhlcmxvbmdsaW5l";
    bench.iter(|| Parser::parse_item(input.as_bytes()).unwrap());
}

fn parsing_list(bench: &mut Bencher) {
    let input = "a, abcdefghigklmnoprst, 123456785686457, 99999999999.999, (), (\"somelongstringvalue\" \"anotherlongstringvalue\";key=:c29tZXZlciBsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXM: 145)";
    bench.iter(|| Parser::parse_list(input.as_bytes()).unwrap());
}

fn parsing_dict(bench: &mut Bencher) {
    let input = "a, dict_key2=abcdefghigklmnoprst, dict_key3=123456785686457, dict_key4=(\"inner-list-member\" :aW5uZXItbGlzdC1tZW1iZXI=:);key=aW5uZXItbGlzdC1wYXJhbWV0ZXJz";
    bench.iter(|| Parser::parse_dictionary(input.as_bytes()).unwrap());
}

fn serializing_item(bench: &mut Bencher) {
    let input = "c29tZXZlcnlsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXNhbnNvbWVvdGhlcmxvbmdsaW5l";
    let parsed_item = Parser::parse_item(input.as_bytes()).unwrap();
    bench.iter(|| parsed_item.serialize_value().unwrap());
}

fn serializing_list(bench: &mut Bencher) {
    let input = "a, abcdefghigklmnoprst, 123456785686457, 99999999999.999, (), (\"somelongstringvalue\" \"anotherlongstringvalue\";key=:c29tZXZlciBsb25nc3RyaW5ndmFsdWVyZXByZXNlbnRlZGFzYnl0ZXM: 145)";
    let parsed_list = Parser::parse_list(input.as_bytes()).unwrap();
    bench.iter(|| parsed_list.serialize_value().unwrap());
}

fn serializing_dict(bench: &mut Bencher) {
    let input = "a, dict_key2=abcdefghigklmnoprst, dict_key3=123456785686457, dict_key4=(\"inner-list-member\" :aW5uZXItbGlzdC1tZW1iZXI=:);key=aW5uZXItbGlzdC1wYXJhbWV0ZXJz";
    let parsed_dict = Parser::parse_dictionary(input.as_bytes()).unwrap();
    bench.iter(|| parsed_dict.serialize_value().unwrap());
}

benchmark_group!(
    benches,
    parsing_item,
    parsing_list,
    parsing_dict,
    serializing_item,
    serializing_list,
    serializing_dict
);
benchmark_main!(benches);
