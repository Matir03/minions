//! UCI protocol implementation

use std::io::{self, Write};

/// Handle a UCI command
pub fn handle_command(cmd: &str) {
    match cmd {
        "uci" => {
            println!("id name Spooky");
            println!("id author Ritam Nag");
            println!("uciok");
            io::stdout().flush().unwrap();
        }
        "isready" => {
            println!("readyok");
            io::stdout().flush().unwrap();
        }
        _ => {
            println!("Unknown command: {}", cmd);
            io::stdout().flush().unwrap();
        }
    }
}
