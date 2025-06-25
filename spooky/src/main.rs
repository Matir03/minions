use std::io::{self, BufRead};
use spooky::{Engine, core::GameConfig};

mod umi;
use umi::protocol::handle_command;
use umi::command::parse_command;

fn main() {
    println!("Spooky - Minions Engine");
    
    let stdin = io::stdin();
    let mut config = GameConfig::default();
    let mut engine = Engine::new(&config);
    
    for line in stdin.lock().lines() {
        let input = line.unwrap();

        if let Some(cmd) = parse_command(&input) {
            let result = handle_command(&cmd, &mut engine);

            match result {
                Ok(Some(new_config)) => {
                    config = new_config;
                    engine = Engine::new(&config);
                }
                Err(err) => {
                    eprintln!("{}", err);
                }
                _ => {}
            }
        }
    }
}
