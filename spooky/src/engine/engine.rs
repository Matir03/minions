use crate::ai::Eval;
use crate::core::{GameAction, GameConfig, GameState, GameTurn, Side, Spell};
use std::sync::Arc;

use super::options::EngineOptions;
use super::search::{search_no_spells, SearchOptions};

use anyhow::{Context, Result};

/// Engine manages the game state and provides methods for game analysis and move generation
pub struct Engine<'a> {
    pub config: &'a GameConfig,
    pub state: GameState<'a>,
    pub options: EngineOptions,
    pub turn: Option<GameTurn>,
}

impl<'a> Engine<'a> {
    /// Create a new engine instance with default options
    pub fn new(config: &'a GameConfig) -> Self {
        Self {
            config,
            state: GameState::new_default(config),
            options: EngineOptions::default(),
            turn: None,
        }
    }

    /// Update the current game state

    pub fn reset_game(&mut self) {
        self.state = GameState::new_default(self.config);
    }

    /// Set engine options
    pub fn set_option(&mut self, name: &str, value: &str) -> Result<()> {
        self.options.set_option(name, value)
    }

    pub fn start_turn(&mut self, spells: Option<Vec<Spell>>) {
        let spells = spells.unwrap_or_else(|| {
            vec![
                if self.options.spells_enabled {
                    Spell::Unknown
                } else {
                    Spell::Blank
                };
                self.config.num_boards + 1
            ]
        });

        self.turn = Some(GameTurn::new(spells, self.config.num_boards));
    }

    pub fn do_action(&mut self, action: GameAction) -> Result<()> {
        let turn = self.turn.as_mut().context("Turn not started")?;
        turn.do_action(action);
        Ok(())
    }

    pub fn take_turn(&mut self, turn: GameTurn) -> Result<Option<Side>> {
        self.state.take_turn(turn)?;
        Ok(self.state.winner())
    }

    // pub fn play<F>(&mut self, search_options: &SearchOptions, buy_spell: F) -> String
    // where
    //     F: FnMut() -> Spell
    // {
    //     if self.options.spells_enabled {
    //         todo!("implement spells");
    //     }

    //     let result = search_no_spells(&self.config, &self.state, search_options);

    //     // Convert the best turn to UMI format
    //     let turn = result.best_turn;
    //     let mut umi_actions = Vec::new();

    //     // For now, generate a simple move action as an example
    //     // In a full implementation, this would parse the actual turn and convert it
    //     umi_actions.push("action boardaction 0 move 1,2 3,4".to_string());
    //     umi_actions.push("action endturn".to_string());

    //     umi_actions.join("\n")
    // }

    /// Start a search with the given options and return the evaluation
    pub fn go(&self, search_options: &SearchOptions) -> (Eval, GameTurn, u32, f64) {
        if self.options.spells_enabled {
            todo!("implement spells");
        }

        let (result, time) = search_no_spells(
            self.config,
            &self.state,
            search_options,
            self.options.heuristic,
        );

        (result.eval, result.best_turn, result.nodes_explored, time)
    }

    pub fn display(&self) {
        // Clears the terminal screen and moves the cursor to the top left.
        print!("\x1B[2J\x1B[1;1H");
        println!("{}", self.state);
    }

    pub fn get_fen(&self) -> Result<String> {
        self.state.to_fen()
    }
}
