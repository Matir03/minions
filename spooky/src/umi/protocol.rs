//! UMI protocol implementation

use anyhow::{bail, ensure, Context, Result};
use spooky::{
    core::{GameAction, GameConfig, GameState, Spell},
    engine::{Engine, SearchOptions},
};
use std::io::{self, Write};

/// Handle a UMI command
pub fn handle_command<'a>(cmd: &str, engine: &mut Engine<'a>) -> Result<Option<GameConfig>> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(None);
    }

    let result = match parts[0] {
        "umi" => {
            println!("id name Spooky author Ritam Nag");
            println!("option name spells type bool default false");
            println!("umiok");
            io::stdout().flush().unwrap();
            Ok(None)
        }
        "isready" => {
            println!("readyok");
            io::stdout().flush().unwrap();
            Ok(None)
        }
        "setoption" => {
            ensure!(
                parts.len() == 4 && parts[1] == "name" && parts[3] == "value",
                "invalid setoption command"
            );

            let option_name = parts[2];
            let option_value = parts[4];

            engine.set_option(option_name, option_value)?;
            Ok(None)
        }
        "config" if parts.len() >= 2 => {
            let fen = parts[1..].join(" ");
            let config = GameConfig::from_fen(&fen)?;
            Ok(Some(config))
        }
        "position" => {
            ensure!(
                parts.len() >= 2,
                "position command requires at least 2 arguments"
            );

            match parts[1] {
                "startpos" => {
                    engine.reset_game();
                    Ok(None)
                }
                "fen" if parts.len() >= 3 => {
                    let fen = parts[2..].join(" ");
                    let state = GameState::from_fen(&fen, &engine.config)?;
                    engine.state = state;
                    Ok(None)
                }
                _ => bail!("invalid position command"),
            }
        }
        cmd @ ("go" | "play") => {
            let args = parts[1..].join(" ");
            let search_options = args.parse::<SearchOptions>()?;

            let (eval, turn, nodes_explored, time) = engine.go(&search_options);
            let winprob = eval.winprob();
            let nps = nodes_explored as f64 / time;

            println!("info eval winprob {}", winprob);
            println!("info nps {} nodes {} time {}", nps, nodes_explored, time);

            // Manually print the turn to customize the endturn line.
            let turn_string = turn.to_string();
            // Using trim_end to handle potential trailing newline before splitting.
            if let Some((body, _)) = turn_string.rsplit_once("endturn") {
                print!("{}", body);
            }

            if cmd == "play" {
                engine.take_turn(turn).unwrap();
            }

            if let Some(winner) = engine.state.winner() {
                println!("endturn winner {}", winner);
            } else {
                println!("endturn");
            }

            Ok(None)
        }
        "turn" => {
            let spells = if parts.len() >= 2 && parts[1] == "spells" {
                let spells = parts[2..]
                    .iter()
                    .map(|s| s.parse::<Spell>())
                    .collect::<Result<Vec<_>>>()?;

                ensure!(
                    spells.len() == engine.config.num_boards + 1,
                    "invalid number of spells"
                );

                Some(spells)
            } else {
                None
            };

            engine.start_turn(spells);
            Ok(None)
        }
        "action" => {
            ensure!(parts.len() >= 2, "missing action arguments");

            let action_name = parts[1];
            let action_args = &parts[2..];

            let action = GameAction::try_from_args(action_name, action_args)?;
            engine.do_action(action)?;
            Ok(None)
        }
        "endturn" => {
            let turn = engine.turn.take().context("Turn not started")?;
            if let Some(winner) = engine.take_turn(turn)? {
                println!("info result winner {}", winner);
            }
            Ok(None)
        }
        "display" => {
            engine.display();
            Ok(None)
        }
        "auto" => {
            let args = parts[1..].join(" ");
            let search_options = args.parse::<SearchOptions>()?;

            engine.display();

            while engine.state.winner().is_none() {
                println!(
                    "\nRunning search for player {:?}...",
                    engine.state.side_to_move
                );
                let (_, turn, _, time) = engine.go(&search_options);
                println!("Best turn found in {:.2}s:\n{}", time, turn);
                engine.take_turn(turn).unwrap();
                if let Some(winner) = engine.state.winner() {
                    println!("Game over! Winner: {}", winner);
                    break;
                }
                engine.display();
            }
            Ok(None)
        }
        // "perft" => {
        //     ensure!(parts.len() >= 3, "perft command requires a depth and at least one board index");
        //     let depth: u32 = parts[1].parse().context("invalid depth")?;
        //     let board_counts = parts[2..].iter()
        //         .map(|s| {
        //             let board_index = s.parse().context("invalid board index")?;
        //             Ok(engine.perft(board_index, depth))
        //         })
        //         .collect::<Result<Vec<u64>>>()?;

        //     println!("perft {}", board_counts.iter().sum::<u64>());
        // }
        "getfen" => {
            println!("{}", engine.get_fen()?);
            Ok(None)
        }
        "quit" => {
            std::process::exit(0);
        }
        cmd => {
            bail!("Unknown command: {}", cmd);
        }
    };
    result
}
