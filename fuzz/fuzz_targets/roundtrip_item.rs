#![no_main]

use libfuzzer_sys::fuzz_target;
use sfv::SerializeValue as _;

fuzz_target!(|item: sfv::Item| {
    let serialized = item.serialize_value().unwrap();
    assert_eq!(
        sfv::Parser::from_bytes(serialized.as_bytes())
            .parse_item()
            .unwrap(),
        item
    );
});
