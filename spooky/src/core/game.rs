//! Game state and rules

use anyhow::{Result, anyhow, bail, ensure, Context};
use std::path::Path;

use crate::core::convert::{FromIndex, ToIndex};
use super::{
    action::GameTurn, 
    board::Board, 
    map::Map, 
    side::{Side, SideArray}, 
    tech::{Tech, TechState, Techline, SPELL_COST},
    spells::Spell,
    units::Unit,
};

/// Static configuration for a Minions game
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameConfig {
    pub num_boards: usize,
    pub points_to_win: i32,
        pub maps: Vec<Map>,
    pub techline: Techline,
    pub start_money: i32,
}

impl Default for GameConfig {
    fn default() -> Self {
        const NUM_BOARDS: usize = 2;
        Self {
            num_boards: NUM_BOARDS,
            points_to_win: 2,
            maps: vec![Map::default(); NUM_BOARDS],
            techline: Techline::default(),
            start_money: 12,
        }
    }
}

impl GameConfig {
    pub fn new(num_boards: usize, points_to_win: i32, maps: Vec<Map>, techline: Techline, start_money: i32) -> Self {
        Self {
            num_boards,
            points_to_win,
            maps,
            techline,
            start_money,
        }
    }

        fn default_maps() -> Vec<Map> {
        vec![Map::default()]
    }

    pub fn spell_cost(&self) -> i32 {
        SPELL_COST * self.num_boards as i32
    }

    /// Convert config to FEN notation
    pub fn to_fen(&self) -> Result<String> {
        let mut fen = String::new();
        
        fen.push_str(&self.num_boards.to_string());
        fen.push(' ');
        fen.push_str(&self.points_to_win.to_string());
        fen.push(' ');
        
        fen.push_str(&self.maps
            .iter()
            .map(|m| m.to_index())
            .collect::<Result<Vec<_>>>()?
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(","));
        fen.push(' ');
        
        fen.push_str(&self.techline.techs.len().to_string());
        fen.push(' ');
        
        fen.push_str(&self.techline.techs
            .iter()
            .map(|t| t.to_index())
            .collect::<Result<Vec<_>>>()?
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(","));
            
        Ok(fen)
    }

    /// Parse config from FEN notation
    pub fn from_fen(fen: &str) -> Result<Self> {
        let mut parts = fen.split_whitespace();
        
        // Parse number of boards
        let num_boards = parts.next()
            .context("Missing number of boards")?
            .parse::<usize>()
            .context("Invalid number of boards")?;
            
        // Parse points to win
        let points_to_win = parts.next()
            .context("Missing points to win")?
            .parse::<i32>()
            .context("Invalid points to win")?;
            
        // Parse maps
        let map_indices = parts.next()
            .context("Missing maps")?
            .split(',')
            .map(|s| s.parse::<usize>().context("Invalid map index"))
            .collect::<Result<Vec<_>>>()?;
            
        let maps = map_indices.into_iter()
            .map(Map::from_index)
            .collect::<Result<Vec<_>>>()?;
            
        // Parse techline length
        let techline_len = parts.next()
            .context("Missing techline length")?
            .parse::<usize>()
            .context("Invalid techline length")?;
            
        // Parse techline
        let tech_indices = parts.next()
            .context("Missing techline")?
            .split(',')
            .map(|s| s.parse::<usize>().context("Invalid tech index"))
            .collect::<Result<Vec<_>>>()?;
            
        ensure!(tech_indices.len() == techline_len, "Invalid techline length");
        
        let techs = tech_indices.into_iter()
            .map(Tech::from_index)
            .collect::<Result<Vec<_>>>()?;
            
        Ok(Self {
            num_boards,
            points_to_win,
            maps,
            techline: Techline { techs },
            start_money: 12, // TODO: parse from FEN?
        })
    }
}

/// State of a Minions game (excluding the static configuration)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState<'a> {
        pub config: &'a GameConfig,
        pub boards: Vec<Board<'a>>,
    pub side_to_move: Side,
    pub ply: i32,
    pub board_points: SideArray<i32>,
    pub tech_state: TechState,
    pub money: SideArray<i32>,
    pub winner: Option<Side>,
}

impl<'a> GameState<'a> {
        pub fn new(config: &'a GameConfig, side_to_move: Side, turn_num: i32, boards: Vec<Board<'a>>, tech_state: TechState, money: SideArray<i32>) -> Self {
        Self {
            config,
            side_to_move,
            ply: turn_num,
            boards,
            board_points: SideArray::new(0, 0),
            tech_state,
            money,
            winner: None,
        }
    }

        pub fn new_default(config: &'a GameConfig) -> Self {
        let boards = config
            .maps
            .iter()
            .map(|map| {
                Board::from_fen(Board::START_FEN, map)
                    .expect("Failed to create board from FEN")
            })
            .collect();
        let start_money = config.start_money;
        Self {
            config,
            side_to_move: Side::S0,
            ply: 1,
            boards,
            board_points: SideArray::new(0, 0),
            tech_state: TechState::new(),
            money: SideArray::new(start_money, start_money),
            winner: None,
        }
    }
    /// Convert state to FEN notation
    pub fn to_fen(&self) -> Result<String> {
        let mut fen = String::new();

        // Board states
        for (i, board) in self.boards.iter().enumerate() {
            if i > 0 {
                fen.push('|');
            }
            fen.push_str(&board.to_fen());
        }

        // Side to move
        fen.push(' ');
        fen.push_str(&self.side_to_move.to_index()?.to_string());

        // Turn number
        fen.push(' ');
        fen.push_str(&self.ply.to_string());

        // Winner
        fen.push(' ');
        let winner_char = match self.winner {
            None => '_',
            Some(Side::S0) => '0',
            Some(Side::S1) => '1',
        };
        fen.push(winner_char);

        // Tech status
        fen.push(' ');
        fen.push_str(&self.tech_state.to_fen());

        // Money
        fen.push(' ');
        fen.push_str(&format!("{}|{}",
            self.money.get(Side::S0)?,
            self.money.get(Side::S1)?
        ));

        Ok(fen)
    }

    /// Parse state from FEN notation
            pub fn from_fen(fen: &str, config: &'a GameConfig) -> Result<Self> {
        let mut parts = fen.split_whitespace();

        // Parse board states
        let board_fens = parts.next()
            .context("Missing board states")?
            .split('|');

        let mut boards = Vec::new();
                for (board_fen, map) in board_fens.zip(&config.maps) {
            boards.push(Board::from_fen(board_fen, map)?);
        }

        ensure!(boards.len() == config.num_boards, "Invalid number of boards");

        // Parse side to move
        let side_idx = parts.next()
            .context("Missing side to move")?
            .parse::<usize>()
            .context("Invalid side to move")?;
        let side_to_move = Side::from_index(side_idx)?;

        // Parse turn number
        let turn_num = parts.next()
            .context("Missing turn number")?
            .parse::<i32>()
            .context("Invalid turn number")?;

        // Parse winner
        let winner_char = parts.next()
            .context("Missing winner")?
            .chars().next().context("Empty winner string")?;
        let winner = match winner_char {
            '_' => None,
            '0' => Some(Side::S0),
            '1' => Some(Side::S1),
            _ => bail!("Invalid winner char"),
        };

        // Parse tech state
        let tech_fen = parts.next()
            .context("Missing tech state")?;
        let tech_state = TechState::from_fen(tech_fen, &config.techline)?;

        // Parse money
        let money_parts: Vec<_> = parts.next()
            .context("Missing money")?
            .split('|')
            .collect();
        ensure!(money_parts.len() == 2, "Invalid money format");

        let money = SideArray::new(
            money_parts[0].parse().context("Invalid money value for side 0")?,
            money_parts[1].parse().context("Invalid money value for side 1")?
        );

        Ok(Self {
            config,
            side_to_move,
            ply: turn_num,
            boards,
            board_points: SideArray::new(0, 0), // Not stored in FEN
            tech_state,
            money,
            winner,
        })
    }

    pub fn end_turn(&mut self) -> Result<()> {
        if self.winner.is_some() {
            return Ok(());
        }

        // End turn on all boards
        for board in &mut self.boards {
            let (income, winner) = board.end_turn(self.side_to_move)?;
            self.money[self.side_to_move] += income;
            
            if let Some(board_winner) = winner {
                self.board_points[board_winner] += 1;
            }
        }

        let points_to_win = self.config.points_to_win;
        if self.board_points[self.side_to_move] >= points_to_win {
            self.winner = Some(self.side_to_move);
        } else if self.board_points[!self.side_to_move] >= points_to_win {
            self.winner = Some(!self.side_to_move);
        }

        if self.winner.is_some() {
            return Ok(());
        }

        // Advance turn
        self.side_to_move = !self.side_to_move;
        self.ply += 1;

        Ok(())
    }

    pub fn take_turn(&mut self, turn: GameTurn) -> Result<()> {
        // Process tech assignments
        let spells_bought = turn.tech_assignment.num_spells() - 1;
        ensure!(spells_bought >= 0, "Must assign all techs");

        let total_spell_cost = spells_bought * (self.boards.len() as i32) * self.config.spell_cost();
        ensure!(self.money[self.side_to_move] >= total_spell_cost);
        self.money[self.side_to_move] -= total_spell_cost;

        self.tech_state.assign_techs(
            turn.tech_assignment,
            self.side_to_move,
            &self.config.techline
        )?;

        // Process spell assignments
        ensure!(turn.spell_assignment.len() == self.boards.len(),
            "Invalid spell_assignment length");
        for (board_idx, spell)
        in turn.spell_assignment.into_iter().enumerate() {
            let board = &mut self.boards[board_idx];
            board.assign_spell(spell, self.side_to_move);
        }

        // Process board actions for each board
        ensure!(turn.board_turns.len() == self.boards.len(),
            "Invalid board_turns length");

        for (board_idx, board_turn) in turn.board_turns.into_iter().enumerate() {
            let board = &mut self.boards[board_idx];
            for action in board_turn.setup_actions {
                board.do_setup_action(self.side_to_move, action)?;
            }
            for action in board_turn.attack_actions {
                board.do_attack_action(self.side_to_move, action)?;
            }
            for action in board_turn.spawn_actions {
                board.do_spawn_action(self.side_to_move, &mut self.money[self.side_to_move], action)?;
            }
        }

        Ok(())
    }

    pub fn winner(&self) -> Option<Side> {
        self.winner
    }
}



#[cfg(test)]
mod tests {
use super::*;
use crate::core::{loc::Loc, units::Unit};

#[test]
fn test_basic_fen_conversion() {
let config = GameConfig::default();
let state = GameState::new_default(&config);

let config_fen = config.to_fen().unwrap();
let new_config = GameConfig::from_fen(&config_fen).unwrap();
assert_eq!(config, new_config);

let state_fen = state.to_fen().unwrap();
let new_state = GameState::from_fen(&state_fen, &config).unwrap();
assert_eq!(state, new_state);
}

#[test]
fn test_empty_spaces_fen() {
    let config = GameConfig::default();
    let boards = config
        .maps
        .iter()
        .map(|map| Board::new(map))
        .collect();
    let state = GameState::new(
        &config,
        Side::S0,
        1,
        boards,
        TechState::new(),
        SideArray::new(config.start_money, config.start_money),
    );

    let state_fen = state.to_fen().unwrap();
    let new_state = GameState::from_fen(&state_fen, &config).unwrap();
    assert_eq!(state, new_state);
}

#[test]
fn test_invalid_fen() {
let config = GameConfig::default();
let valid_board_fen = "0/0/0/0/0/0/0/0/0/0|0/0/0/0/0/0/0/0/0/0";
let valid_tech_fen = "LLLLLLLLLLLLLLLLLLLLLLLLLL|LLLLLLLLLLLLLLLLLLLLLLLLLL";

// Not enough boards
let fen1 = format!("0/0/0/0/0/0/0/0/0/0 0 1 _ {} 10|5", valid_tech_fen);
assert!(GameState::from_fen(&fen1, &config).is_err());

// Too many boards
let fen2 = format!("0|0|0 0 1 _ {} 10|5", valid_tech_fen);
assert!(GameState::from_fen(&fen2, &config).is_err());

// Invalid side to move
let fen3 = format!("{} 2 1 _ {} 10|5", valid_board_fen, valid_tech_fen);
assert!(GameState::from_fen(&fen3, &config).is_err());

// Invalid winner
let fen4 = format!("{} 0 1 X {} 10|5", valid_board_fen, valid_tech_fen);
assert!(GameState::from_fen(&fen4, &config).is_err());

// Invalid money
let fen5 = format!("{} 0 1 _ {} 10", valid_board_fen, valid_tech_fen);
assert!(GameState::from_fen(&fen5, &config).is_err());

// Invalid tech state
let fen6 = format!("{} 0 1 _ L|L 10|5", valid_board_fen);
assert!(GameState::from_fen(&fen6, &config).is_err());
}

#[test]
fn test_win_condition() {
    let config = GameConfig::default();
    let mut state = GameState::new_default(&config);
    state.board_points[Side::S0] = config.points_to_win;

    state.end_turn().unwrap();
    assert_eq!(state.winner(), Some(Side::S0));
}

#[test]
fn test_load_fen_from_file() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let config_path = Path::new(manifest_dir).join("data/test_config.fen");
    let state_path = Path::new(manifest_dir).join("data/test_state.fen");

    let config_fen = std::fs::read_to_string(config_path).unwrap();
    let state_fen = std::fs::read_to_string(state_path).unwrap();

    let config = GameConfig::from_fen(&config_fen).unwrap();
    let state = GameState::from_fen(&state_fen, &config).unwrap();

    assert_eq!(config.num_boards, 2);
    assert_eq!(config.points_to_win, 2);
    assert_eq!(state.side_to_move, Side::S0);
    assert_eq!(state.ply, 1);
    assert_eq!(state.money[Side::S0], 12);
    assert_eq!(state.money[Side::S1], 12);
}
}