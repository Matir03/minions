//! Game state and rules

use anyhow::{Result, anyhow, bail, ensure, Context};
use crate::core::convert::{FromIndex, ToIndex};
use super::{
    action::GameTurn, 
    board::Board, 
    map::Map, 
    phase::Phase,
    side::{Side, SideArray}, 
    tech::{Tech, TechState, Techline, SPELL_COST},
    spells::Spell,
    units::Unit,
};

/// Static configuration for a Minions game
#[derive(Debug, Clone)]
pub struct GameConfig {
    pub num_boards: usize,
    pub points_to_win: i32,
    pub maps: Vec<Map>,
    pub techline: Techline,
    pub start_money: i32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            num_boards: 2,
            points_to_win: 2,
            maps: vec![Map::BlackenedShores, Map::MidnightLake],
            techline: Techline::default(),
            start_money: 12,
        }
    }
}

impl GameConfig {
    pub fn spell_cost(&self) -> i32 {
        SPELL_COST * self.num_boards as i32
    }

    /// Convert config to FEN notation
    pub fn to_fen(&self) -> Result<String> {
        let mut fen = String::new();
        
        fen.push_str(&self.num_boards.to_string());
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
#[derive(Debug, Clone)]
pub struct GameState {
    pub boards: Vec<Board>,
    pub side_to_move: Side,
    pub ply: i32,
    pub board_points: SideArray<i32>,
    pub tech_state: TechState,
    pub money: SideArray<i32>,
    pub winner: Option<Side>,
}

impl Default for GameState {
    fn default() -> Self {
        let config = GameConfig::default();
        let boards = config.maps.iter().map(|&map| Board::default()).collect();

        Self {
            side_to_move: Side::S0,
            ply: 1,
            boards,
            board_points: SideArray::new(0, 0),
            tech_state: TechState::new(),
            money: SideArray::new(config.start_money, config.start_money),
            winner: None,
        }
    }
}

impl GameState {
    pub fn new(side_to_move: Side, turn_num: i32, boards: Vec<Board>, tech_state: TechState, money: SideArray<i32>) -> Self {
        Self {
            side_to_move,
            ply: turn_num,
            boards,
            board_points: SideArray::new(0, 0),
            tech_state,
            money,
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
    pub fn from_fen(fen: &str, config: &GameConfig) -> Result<Self> {
        let mut parts = fen.split_whitespace();

        // Parse board states
        let board_fens = parts.next()
            .context("Missing board states")?
            .split('|');

        let mut boards = Vec::new();
        for (board_fen, &map) in board_fens.zip(&config.maps) {
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
            side_to_move,
            ply: turn_num,
            boards,
            board_points: SideArray::new(0, 0), // Not stored in FEN
            tech_state,
            money,
            winner,
        })
    }

    pub fn end_turn(&mut self, config: &GameConfig) -> Result<()> {
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

        let points_to_win = config.points_to_win;
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

    pub fn take_turn(&mut self, turn: GameTurn, config: &GameConfig) -> Result<()> {
        // Process tech assignments
        let spells_bought = turn.tech_assignment.num_spells() - 1;
        ensure!(spells_bought >= 0, "Must assign all techs");

        let total_spell_cost = spells_bought * (self.boards.len() as i32) * SPELL_COST;
        ensure!(self.money[self.side_to_move] >= total_spell_cost);
        self.money[self.side_to_move] -= total_spell_cost;

        self.tech_state.assign_techs(
            turn.tech_assignment,
            self.side_to_move,
            &config.techline
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

/// Parse game state from FEN notation
pub fn parse_fen(fen_str: &str) -> Result<(GameConfig, GameState)> {
    let mut parts_iter = fen_str.split_whitespace();
    
    // First 5 parts are for GameConfig
    let config_fen_string_parts: Vec<&str> = parts_iter.by_ref().take(5).collect();
    ensure!(config_fen_string_parts.len() == 5, "Invalid FEN: Not enough parts for GameConfig - expected 5, got {}", config_fen_string_parts.len());
    let config_fen_to_parse = config_fen_string_parts.join(" ");
    let config = GameConfig::from_fen(&config_fen_to_parse)?;

    // The rest of the parts are for GameState
    let state_fen_string_parts: Vec<&str> = parts_iter.collect();
    ensure!(!state_fen_string_parts.is_empty(), "Invalid FEN: No parts remaining for GameState");
    let state_fen_to_parse = state_fen_string_parts.join(" ");
    let state = GameState::from_fen(&state_fen_to_parse, &config)?;
    
    Ok((config, state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{loc::Loc, units::Unit};

    #[test]
    fn test_basic_fen_conversion() {
        let fen = "1 2 0 4 1,2,3,4 Z8i/10/10/10/10/10/10/10/10/10 0 1 _ LLLU|LLLL 10|5";
        let (config, state) = parse_fen(fen).unwrap();

        assert_eq!(config.num_boards, 1);
        assert_eq!(config.points_to_win, 2);
        assert_eq!(config.maps.len(), 1);
        assert_eq!(config.techline.techs.len(), 4);
        assert_eq!(state.side_to_move, Side::S0);
        assert_eq!(state.ply, 1);
        assert_eq!(state.winner, None);
        assert_eq!(state.money[Side::S0], 10);
        assert_eq!(state.money[Side::S1], 5);
    }

    #[test]
    fn test_empty_spaces_fen() {
        let fen = "2 10 0,1 4 1,2,3,4 10/10/10/10/10/10/10/10/10/10|10/10/10/10/10/10/10/10/10/10 1 1 _ LLLU|LLLL 10|5";
        let (config, state) = parse_fen(fen).unwrap();

        assert_eq!(config.num_boards, 2);
        assert_eq!(state.boards.len(), 2);
        assert_eq!(state.side_to_move, Side::S1);
    }

    // #[test]
    // fn test_fen_with_modifiers() {
    //     let fen = "1 0 4 1,2,3,4 Z8i/0/0/0/0/0/0/0/0/0 0 LLLUUA|LLLLLA 10|5";
    //     let (_, state) = Game::parse_fen(fen).unwrap();

    //     let piece = state.boards[0].get_piece(&Loc { y: 0, x: 0 }).unwrap();
    //     assert_eq!(piece.side, Side::S0);
    //     assert_eq!(piece.unit, Unit::Zombie);
    //     assert!(!piece.modifiers.shielded);
    // }

    #[test]
    fn test_invalid_fen() {
        // Invalid number of boards
        assert!(parse_fen("0 0 4 0,1,2,3 0/0/0/0/0/0/0/0/0/0 0 LLLUUA|LLLLLA 10|5").is_err());

        // Invalid tech status
        assert!(parse_fen("1 0 4 0,1,2,3 0/0/0/0/0/0/0/0/0/0 0 LLLUUX|LLLLLA 10|5").is_err());

        // Invalid money format
        assert!(parse_fen("1 0 4 0,1,2,3 0/0/0/0/0/0/0/0/0/0 0 LLLUUA|LLLLLA 10").is_err());
    }

    #[test]
    fn test_win_condition() {
        let mut state = GameState::default();
        let config = GameConfig::default();
        state.board_points[Side::S0] = config.points_to_win;

        state.end_turn(&config).unwrap();

        assert_eq!(state.winner(), Some(Side::S0));
    }
}