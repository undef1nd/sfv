#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = sfv::Parser::from_bytes(data).parse_dictionary();
});
