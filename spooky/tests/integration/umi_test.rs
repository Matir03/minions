use spooky::umi::{command, protocol};

#[test]
fn test_umi_command_parsing() {
    let cmd = command::parse_command("umi");
    assert_eq!(cmd, Some("umi".to_string()));
}

#[test]
fn test_umi_protocol() {
    // TODO: Add tests for UMI protocol handling
}
