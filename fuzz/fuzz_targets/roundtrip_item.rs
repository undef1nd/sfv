#![no_main]

use libfuzzer_sys::fuzz_target;
use sfv::FieldType as _;

fuzz_target!(|item: sfv::Item| {
    let serialized = item.serialize();
    assert_eq!(
        sfv::Parser::new(&serialized).parse::<sfv::Item>().unwrap(),
        item
    );
});
