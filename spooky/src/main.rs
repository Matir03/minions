use spooky::{core::GameConfig, Engine};
use std::io::{self, BufRead};

mod umi;
use umi::command::parse_command;
use umi::protocol::handle_command;

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
                    if engine.options.strict_mode {
                        panic!("{}", err);
                    } else {
                        eprintln!("{}", err);
                    }
                }
                _ => {}
            }
        }
    }
}
