//! Game state and rules

use anyhow::{anyhow, bail, ensure, Context, Result};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    action::GameTurn,
    board::Board,
    map::Map,
    side::{Side, SideArray},
    spells::Spell,
    tech::{Tech, TechState, Techline, SPELL_COST},
    units::Unit,
};
use crate::core::convert::{FromIndex, ToIndex};

const START_MONEY_PER_BOARD: i32 = 6;

/// Static configuration for a Minions game
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameConfig {
    pub num_boards: usize,
    pub points_to_win: i32,
    pub maps: Vec<Map>,
    pub techline: Techline,
}

pub(crate) fn generate_game_id() -> String {
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("game_{}", ts_ms)
}

impl Default for GameConfig {
    fn default() -> Self {
        const NUM_BOARDS: usize = 2;
        Self {
            num_boards: NUM_BOARDS,
            points_to_win: 2,
            maps: vec![Map::BlackenedShores, Map::MidnightLake],
            techline: Techline::default(),
        }
    }
}

impl GameConfig {
    pub fn new(num_boards: usize, points_to_win: i32, maps: Vec<Map>, techline: Techline) -> Self {
        Self {
            num_boards,
            points_to_win,
            maps,
            techline,
        }
    }

    fn default_maps() -> Vec<Map> {
        vec![Map::default()]
    }

    pub fn spell_cost(&self) -> i32 {
        SPELL_COST * self.num_boards as i32
    }
}

/// State of a Minions game (excluding the static configuration)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameState<'a> {
    pub config: &'a GameConfig,
    pub game_id: String,
    pub boards: Vec<Board<'a>>,
    pub side_to_move: Side,
    pub ply: i32,
    pub board_points: SideArray<i32>,
    pub tech_state: TechState,
    pub money: SideArray<i32>,
    pub winner: Option<Side>,
}

impl<'a> GameState<'a> {
    pub fn new(
        config: &'a GameConfig,
        side_to_move: Side,
        turn_num: i32,
        boards: Vec<Board<'a>>,
        tech_state: TechState,
        money: SideArray<i32>,
    ) -> Self {
        Self {
            config,
            game_id: generate_game_id(),
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
                Board::from_fen(Board::START_FEN, map).expect("Failed to create board from FEN")
            })
            .collect();
        Self {
            config,
            game_id: generate_game_id(),
            side_to_move: Side::Yellow,
            ply: 1,
            boards,
            board_points: SideArray::new(0, 0),
            tech_state: TechState::new(),
            money: SideArray::new(0, config.num_boards as i32 * START_MONEY_PER_BOARD),
            winner: None,
        }
    }

    pub fn end_turn(&mut self) -> Result<()> {
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
        if self.winner.is_some() {
            bail!("Game is already over");
        }

        // Process tech assignments
        let spells_bought = (turn.tech_assignment.num_spells() - 1).max(0);
        let total_spell_cost = spells_bought * self.config.spell_cost();
        ensure!(self.money[self.side_to_move] >= total_spell_cost);
        self.money[self.side_to_move] -= total_spell_cost;

        self.tech_state.assign_techs(
            turn.tech_assignment,
            self.side_to_move,
            &self.config.techline,
        )?;

        // Process spell assignments
        ensure!(
            turn.spell_assignment.len() == self.boards.len(),
            "Invalid spell_assignment length"
        );
        for (board_idx, spell) in turn.spell_assignment.into_iter().enumerate() {
            let board = &mut self.boards[board_idx];
            board.assign_spell(spell, self.side_to_move);
        }

        // Process board actions for each board
        ensure!(
            turn.board_turns.len() == self.boards.len(),
            "Invalid board_turns length"
        );

        for (board_idx, board_turn) in turn.board_turns.into_iter().enumerate() {
            let board = &mut self.boards[board_idx];
            let (money, rebate) = board.take_turn(
                self.side_to_move,
                board_turn,
                self.money[self.side_to_move],
                &self.tech_state,
            )?;
            self.money[self.side_to_move] = money;
            self.money[!self.side_to_move] += rebate;
        }
        self.end_turn();

        Ok(())
    }

    pub fn winner(&self) -> Option<Side> {
        self.winner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fen_round_trip() {
        let config = GameConfig::default();
        let state = GameState::new_default(&config);
        let fen = state.to_fen().expect("to_fen failed");
        let parsed = GameState::from_fen(&fen, &config).expect("from_fen failed");
        let fen2 = parsed.to_fen().expect("to_fen failed after parse");
        assert_eq!(fen, fen2, "FEN round-trip mismatch: {} != {}", fen, fen2);
    }
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
        let boards = config.maps.iter().map(|map| Board::new(map)).collect();
        let state = GameState::new(
            &config,
            Side::Yellow,
            1,
            boards,
            TechState::new(),
            SideArray::new(0, config.num_boards as i32 * START_MONEY_PER_BOARD),
        );

        let state_fen = state.to_fen().unwrap();
        let new_state = GameState::from_fen(&state_fen, &config).unwrap();
        assert_eq!(state, new_state);
    }

    #[test]
    fn test_invalid_fen() {
        let config = GameConfig::default();
        let valid_board_fen = "f|||||0/0/0/0/0/0/0/0/0/0 f|||||0/0/0/0/0/0/0/0/0/0";
        let valid_tech_fen = "LLLLLLLLLLLLLLLLLLLLLLLLL|LLLLLLLLLLLLLLLLLLLLLLLLL";

        // Not enough boards
        let fen1 = format!("10|5 0|0 {} f|||||0/0/0/0/0/0/0/0/0/0 0 1", valid_tech_fen);
        assert!(GameState::from_fen(&fen1, &config).is_err());

        // Too many boards
        let fen2 = format!("10|5 0|0 {} f|||||0/0/0/0/0/0/0/0/0/0 f|||||0/0/0/0/0/0/0/0/0/0 f|||||0/0/0/0/0/0/0/0/0/0 0 1", valid_tech_fen);
        assert!(GameState::from_fen(&fen2, &config).is_err());

        // Invalid side to move
        let fen3 = format!("10|5 0|0 {} {} 10|5 1", valid_tech_fen, valid_board_fen); // 5th part should be board2, but here we have 10|5
        assert!(GameState::from_fen(&fen3, &config).is_err());

        // Invalid winner/side to move (it's called side_to_move now)
        let fen4 = format!("10|5 0|0 {} {} X 1", valid_tech_fen, valid_board_fen);
        assert!(GameState::from_fen(&fen4, &config).is_err());

        // Invalid money
        let fen5 = format!("10 0|0 {} {} 0 1", valid_tech_fen, valid_board_fen);
        assert!(GameState::from_fen(&fen5, &config).is_err());

        // Invalid tech state
        let fen6 = format!("{} 0 1 _ L|L 10|5", valid_board_fen);
        assert!(GameState::from_fen(&fen6, &config).is_err());
    }

    #[test]
    fn test_win_condition() {
        let config = GameConfig::default();
        let mut state = GameState::new_default(&config);
        state.board_points[Side::Yellow] = config.points_to_win;

        state.end_turn().unwrap();
        assert_eq!(state.winner(), Some(Side::Yellow));
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
        assert_eq!(state.side_to_move, Side::Yellow);
        assert_eq!(state.ply, 1);
        assert_eq!(state.money[Side::Yellow], 12);
        assert_eq!(state.money[Side::Blue], 12);
    }
}
