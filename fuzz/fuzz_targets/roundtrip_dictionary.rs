#![no_main]

use libfuzzer_sys::fuzz_target;
use sfv::FieldType as _;

fuzz_target!(|dict: sfv::Dictionary| {
    let serialized = dict.serialize();
    if dict.is_empty() {
        assert!(serialized.is_none());
    } else {
        assert_eq!(
            sfv::Parser::new(&serialized.unwrap())
                .parse::<sfv::Dictionary>()
                .unwrap(),
            dict
        );
    }
});
