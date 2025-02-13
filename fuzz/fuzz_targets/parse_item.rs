#![no_main]

use libfuzzer_sys::fuzz_target;
use sfv::SerializeValue as _;

fuzz_target!(|data: &[u8]| {
    if let Ok(item) = sfv::Parser::from_bytes(data).parse_item() {
        let serialized = item.serialize_value().unwrap();
        assert_eq!(
            sfv::Parser::from_bytes(serialized.as_bytes())
                .parse_item()
                .unwrap(),
            item
        );
    }
});
