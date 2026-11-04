#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(arbitrary::Arbitrary, Debug)]
struct Input<'a> {
    data: &'a [u8],
    version: sfv::Version,
}

fuzz_target!(|input: Input| {
    let parser = || sfv::Parser::new(input.data).with_version(input.version);

    let _ = parser().parse_dictionary_with_visitor(sfv::visitor::Ignored);
    let _ = parser().parse_list_with_visitor(sfv::visitor::Ignored);
    let _ = parser().parse_item_with_visitor(sfv::visitor::Ignored);
});
