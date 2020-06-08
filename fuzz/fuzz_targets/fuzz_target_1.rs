#![no_main]
#[macro_use] use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > 0 {
        let _ = structured_headers::parser::Parser::parse(&data, "dictionary");
    }
});
