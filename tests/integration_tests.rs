use sfv::{ListEntry, Parser};
use std::error::Error;

#[test]
fn test_report_to_header() -> Result<(), Box<dyn Error>> {
    // cross-origin-embedder-policy: require-corp; report-to="coep"
    let coep = br#"require-corp; report-to="coep""#;
    let endpoints = br#"csp="https://example.com/csp-reports", hpkp="https://example.com/hpkp-reports", coep="https://example.com/coep""#;

    let coep_parsed = Parser::from_bytes(coep).parse_item()?;
    let token = coep_parsed
        .bare_item
        .as_token()
        .ok_or("unexpected BareItem variant")?;
    assert_eq!(token.as_str(), "require-corp");

    let coep_endpoint = coep_parsed
        .params
        .get("report-to")
        .ok_or("parameter does not exist")?
        .as_str()
        .ok_or("unexpected BareItem variant")?;

    let endpoints_parsed = Parser::from_bytes(endpoints).parse_dictionary()?;
    if let Some(ListEntry::Item(item)) = endpoints_parsed.get(coep_endpoint) {
        let item_value = item
            .bare_item
            .as_str()
            .ok_or("unexpected BareItem variant")?;
        assert_eq!(item_value, "https://example.com/coep");
        return Ok(());
    }
    Err("unexpected endpoint value".into())
}
