#[derive(arbitrary::Arbitrary, Debug)]
pub struct Input<'a> {
    pub data: &'a [u8],
    pub version: sfv::Version,
}
