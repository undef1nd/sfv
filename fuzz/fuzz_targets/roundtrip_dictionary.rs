#![no_main]

use libfuzzer_sys::fuzz_target;
use sfv::SerializeValue as _;

fuzz_target!(|dict: sfv::Dictionary| {
    let serialized = dict.serialize_value();
    if dict.is_empty() {
        assert!(serialized.is_err());
    } else {
        assert_eq!(
            sfv::Parser::from_bytes(serialized.unwrap().as_bytes())
                .parse_dictionary()
                .unwrap(),
            dict
        );
    }
});
