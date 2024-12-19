use spooky::core::game::{Game, GameConfig, GameState};
use std::io::{self, BufRead};

mod uci;
use uci::protocol::handle_command;
use uci::command::parse_command;

fn main() {
    println!("Spooky - Minions Engine");
    
    let stdin = io::stdin();
    let config = GameConfig::default();
    let state = GameState::default();
    let _game = Game::new(&config, state);
    
    for line in stdin.lock().lines() {
        let input = line.unwrap();
        if let Some(cmd) = parse_command(&input) {
            handle_command(&cmd);
        }
    }
}
