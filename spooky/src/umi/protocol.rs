//! UMI protocol implementation

use std::io::{self, Write};
use anyhow::{bail, ensure, Context, Result};
use spooky::{
    core::{
        GameConfig,
        GameState,
        GameAction,
        parse_fen,
        Spell,
    },
    engine::{Engine, SearchOptions}
};

/// Handle a UMI command
pub fn handle_command(cmd: &str, engine: &mut Engine) -> Result<()> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(());
    }

    match parts[0] {
        "umi" => {
            println!("id name Spooky author Ritam Nag");
            println!("option name spells type bool default false");
            println!("umiok");
            io::stdout().flush().unwrap();
        }
        "isready" => {
            println!("readyok");
            io::stdout().flush().unwrap();
        }
        "setoption" => {
            ensure!(parts.len() == 4 && parts[1] == "name" && parts[3] == "value",
                "invalid setoption command");

            let option_name = parts[2];
            let option_value = parts[4];

            engine.set_option(option_name, option_value)?;
        }
        "position" => {
            ensure!(parts.len() >= 2, "position command requires at least 2 arguments");

            match parts[1] {
                "startpos" => {
                    engine.reset_game();
                }
                "config" if parts.len() >= 3 => {
                    let fen = parts[2..].join(" ");
                    let config = GameConfig::from_fen(&fen)?;

                    engine.set_game(config, GameState::default());
                }
                "fen" if parts.len() >= 3 => {
                    let fen = parts[2..].join(" ");
                    let (config, state) = parse_fen(&fen)?;

                    engine.set_game(config, state);
                }
                _ => bail!("invalid position command")
            }
        }
        "go" => {
            let args = parts[1..].join(" ");
            let search_options = args.parse::<SearchOptions>()?;

            let eval = engine.go(&search_options);
            let winprob = eval.winprob();

            println!("info eval winprob {}", winprob);
        }
        "play" => {
            let args = parts[1..].join(" ");
            let search_options = args.parse::<SearchOptions>()?;

            let turn_response = engine.play(&search_options, || {
                todo!("implement spell buying communication");
            });

            println!("{}", turn_response);
        }
        "turn" => {
            let spells = if parts.len() >= 2 {
                ensure!(parts[1] == "spells", "invalid turn arguments");

                let spells = parts[2..].iter()
                    .map(|s| s.parse::<Spell>())
                    .collect::<Result<Vec<_>>>()?;

                ensure!(spells.len() == engine.config.num_boards + 1,
                    "invalid number of spells");

                Some(spells)
            } else {
                None
            };

            engine.start_turn(spells);
        }
        "action" => {
            ensure!(parts.len() >= 2, "missing action arguments");

            let action_name = parts[1];
            let action_args = &parts[2..];

            let action = GameAction::from_args(action_name, action_args)?;
            engine.do_action(action)?;
        }
        "endturn" => {
            let winner = engine.end_turn()?;

            if let Some(winner) = winner {
                println!("info result winner {}", winner);
            }
        }
        "display" => {
            engine.display();
        }
        "perft" => {
            ensure!(parts.len() >= 2, "perft command requires at least 2 arguments");
            let board_indices = parts[1..].iter()
                .map(|s| Ok(engine.perft(
                    s.parse().context("invalid board index")?))
                )
                .collect::<Result<Vec<u64>>>()?;

            println!("perft {}", board_indices.iter().sum::<u64>());
        }
        "getfen" => {
            println!("{}", engine.get_fen()?);
        }
        "quit" => {
            std::process::exit(0);
        }
        cmd => {
            bail!("Unknown command: {}", cmd);
        }
    }

    Ok(())
}
