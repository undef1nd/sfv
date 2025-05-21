#![no_main]

mod input;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: input::Input| {
    let _ = sfv::Parser::new(input.data)
        .with_version(input.version)
        .parse::<sfv::Dictionary>();
});
