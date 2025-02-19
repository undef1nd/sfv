#![no_main]

use libfuzzer_sys::fuzz_target;
use sfv::SerializeValue as _;

fuzz_target!(|data: &[u8]| {
    if let Ok(list) = sfv::Parser::from_bytes(data).parse_list() {
        let serialized = list.serialize_value();
        if list.is_empty() {
            assert!(serialized.is_err());
        } else {
            assert_eq!(
                sfv::Parser::from_bytes(serialized.unwrap().as_bytes()).parse_list(),
                Ok(list)
            );
        }
    }
});
