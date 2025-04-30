#![no_main]

use libfuzzer_sys::fuzz_target;
use sfv::SerializeValue as _;

fuzz_target!(|dict: sfv::Dictionary| {
    let serialized = dict.serialize_value();
    if dict.is_empty() {
        assert!(serialized.is_none());
    } else {
        assert_eq!(
            sfv::Parser::new(&serialized.unwrap())
                .parse_dictionary()
                .unwrap(),
            dict
        );
    }
});
