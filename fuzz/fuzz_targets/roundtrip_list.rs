#![no_main]

use libfuzzer_sys::fuzz_target;
use sfv::SerializeValue as _;

fuzz_target!(|list: sfv::List| {
    let serialized = list.serialize_value();
    if list.is_empty() {
        assert!(serialized.is_err());
    } else {
        assert_eq!(
            sfv::Parser::from_bytes(serialized.unwrap().as_bytes())
                .parse_list()
                .unwrap(),
            list,
        );
    }
});
