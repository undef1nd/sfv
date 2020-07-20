use sfv::{BareItem, Dictionary, Item, ListEntry, Parser};
use std::error::Error;

#[test]
fn test_report_to_header() -> Result<(), Box<dyn Error>> {
    // cross-origin-embedder-policy: require-corp; report-to="coep"
    let coep = br#"require-corp; report-to="coep""#;
    let endpoints = br#"csp="https://example.com/csp-reports", hpkp="https://example.com/hpkp-reports", coep="https://example.com/coep""#;

    let coep_parsed = Parser::parse_item(coep)?;
    let token = match coep_parsed.bare_item {
        BareItem::Token(val) => val,
        _ => return Err("can't unwrap bare_item".into()),
    };
    assert_eq!(token, "require-corp");

    let coep_endpoint = match coep_parsed.params.get("report-to") {
        Some(BareItem::String(val)) => val,
        _ => return Err("unexpected param value".into()),
    };

    let endpoints_parsed = Parser::parse_dictionary(endpoints)?;

    if let Some(ListEntry::Item(itm)) = endpoints_parsed.get(coep_endpoint) {
        if let BareItem::String(ref val) = itm.bare_item {
            assert_eq!(val, "https://example.com/coep");
            return Ok(());
        }
    }
    Err("unexpected endpoint value".into())
}
