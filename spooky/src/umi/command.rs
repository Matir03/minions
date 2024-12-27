//! UMI command parsing

/// Parse a UMI command string
pub fn parse_command(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    Some(input.to_string())
}
