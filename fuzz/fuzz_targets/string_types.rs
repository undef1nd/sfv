#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = sfv::KeyRef::from_bytes(data);
    let _ = sfv::TokenRef::from_bytes(data);
    let _ = sfv::StringRef::from_bytes(data);
});
