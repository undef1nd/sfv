#![no_main]

use libfuzzer_sys::fuzz_target;
use sfv::FieldType as _;

fuzz_target!(|list: sfv::List| {
    let serialized = list.serialize();
    if list.is_empty() {
        assert!(serialized.is_none());
    } else {
        assert_eq!(
            sfv::Parser::new(&serialized.unwrap())
                .parse::<sfv::List>()
                .unwrap(),
            list,
        );
    }
});
