use spooky::Game;
use std::io::{self, BufRead};

mod uci;
use uci::protocol::handle_command;
use uci::command::parse_command;

fn main() {
    println!("Spooky - Minions Engine");
    
    let stdin = io::stdin();
    let _game = Game::new();
    
    for line in stdin.lock().lines() {
        let input = line.unwrap();
        if let Some(cmd) = parse_command(&input) {
            handle_command(&cmd);
        }
    }
}
