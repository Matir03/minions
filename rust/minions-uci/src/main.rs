use minions_engine::Game;
use std::io::{self, BufRead, Write};

fn main() {
    println!("Minions Engine");
    
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let game = Game::new();
    
    for line in stdin.lock().lines() {
        let input = line.unwrap();
        let cmd = input.trim();
        
        match cmd {
            "quit" => break,
            "uci" => {
                println!("id name Minions Engine");
                println!("id author");
                println!("uciok");
                stdout.flush().unwrap();
            }
            "isready" => {
                println!("readyok");
                stdout.flush().unwrap();
            }
            _ => {
                println!("Unknown command: {}", cmd);
                stdout.flush().unwrap();
            }
        }
    }
}
