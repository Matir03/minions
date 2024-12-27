use std::io::{self, BufRead};
use spooky::Engine;

mod umi;
use umi::protocol::handle_command;
use umi::command::parse_command;

fn main() {
    println!("Spooky - Minions Engine");
    
    let stdin = io::stdin();
    let mut engine = Engine::new();
    
    for line in stdin.lock().lines() {
        let input = line.unwrap();

        if let Some(cmd) = parse_command(&input) {
            let result = handle_command(&cmd, &mut engine);

            if let Err(err) = result {
                eprintln!("{}", err);
            }
        }
    }
}
