use crate::core::{GameConfig, GameState, GameAction, Spell, Side, Turn};
use crate::general::Eval;

use super::options::EngineOptions;
use super::search::{search_no_spells, SearchOptions};

use anyhow::{Context, Result};

/// Engine manages the game state and provides methods for game analysis and move generation
pub struct Engine {
    pub config: GameConfig,
    pub state: GameState,
    pub options: EngineOptions,
    pub turn: Option<Turn>,
}

impl Engine {
    /// Create a new engine instance with default options
    pub fn new() -> Self {
        Self {
            config: GameConfig::default(),
            state: GameState::default(),
            options: EngineOptions::default(),
            turn: None,
        }
    }

    /// Update the current game state
    pub fn set_game(&mut self, config: GameConfig, game: GameState) {
        self.config = config;
        self.state = game;
    }

    pub fn reset_game(&mut self) {
        self.state = GameState::default();
    }

    /// Set engine options
    pub fn set_option(&mut self, name: &str, value: &str) -> Result<()> {
        self.options.set_option(name, value)
    }

    pub fn start_turn(&mut self, spells: Option<Vec<Spell>>) {
        let spells = spells.unwrap_or_else(|| vec![
            if self.options.spells_enabled { Spell::Unknown } else { Spell::Blank }; 
            self.config.num_boards + 1
        ]);
      
        self.turn = Some(Turn::new(spells, self.config.num_boards));
    }

    pub fn do_action(&mut self, action: GameAction) -> Result<()> {
        let turn = self.turn.as_mut().context("Turn not started")?;
        turn.do_action(action);
        Ok(())
    }

    pub fn end_turn(&mut self) -> Result<Option<Side>> {
        let turn = self.turn.take().context("Turn not started")?;
        self.take_turn(turn)
    }

    pub fn take_turn(&mut self, turn: Turn) -> Result<Option<Side>> {
        self.state.take_turn(turn, &self.config)
    }

    pub fn play<F>(&mut self, search_options: &SearchOptions, buy_spell: F) -> Turn 
    where 
        F: FnMut() -> Spell
    {
        if self.options.spells_enabled {
            todo!("implement spells");
        }

        let node = search_no_spells(&self.config, &self.state, search_options);

        node.best_turn()
    }

    /// Start a search with the given options and return the evaluation
    pub fn go(&self, search_options: &SearchOptions) -> Eval {
        if self.options.spells_enabled {
            todo!("implement spells");
        }

        let node = search_no_spells(&self.config, &self.state, search_options);

        node.eval()       
    }

    pub fn display(&self) {
        println!("{}", self.state);
    }

    pub fn get_fen(&self) -> Result<String> {
        self.state.to_fen()
    }

    pub fn perft(&self, board_index: usize) -> u64 {
        todo!()
    }
}
