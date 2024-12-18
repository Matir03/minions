use spooky::uci::{command, protocol};

#[test]
fn test_uci_command_parsing() {
    let cmd = command::parse_command("uci");
    assert_eq!(cmd, Some("uci".to_string()));
}

#[test]
fn test_uci_protocol() {
    // TODO: Add tests for UCI protocol handling
}
