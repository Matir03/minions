//! Game state and rules

use anyhow::{Result, anyhow, bail, ensure};
use crate::core::convert::{FromIndex, ToIndex};
use super::{
    action::Turn, 
    board::Board, 
    map::Map, 
    side::{Side, SideArray}, 
    tech::{Tech, TechState, Techline, SPELL_COST}
};

/// Static configuration for a Minions game
#[derive(Debug, Clone)]
pub struct GameConfig {
    pub num_boards: usize,
    pub points_to_win: i32,
    pub maps: Vec<Map>,
    pub techline: Techline,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            num_boards: 1,
            points_to_win: 1,
            maps: vec![Map::BlackenedShores],
            techline: Techline::default(),
        }
    }
}

/// State of a Minions game (excluding the static configuration)
#[derive(Debug, Clone)]
pub struct GameState {
    pub side_to_move: Side,
    pub boards: Vec<Board>,
    pub board_points: SideArray<i32>,
    pub tech_state: TechState,
    pub money: SideArray<i32>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            side_to_move: Side::S0,
            boards: vec![Board::default()],
            board_points: SideArray::new(0, 0),
            tech_state: TechState::new(),
            money: SideArray::new(0, 6),
        }
    }
}

/// Represents the state of a Minions game
#[derive(Debug, Clone)]
pub struct Game<'g> {
    pub config: &'g GameConfig,
    pub state: GameState,
}

impl<'g> Game<'g> {
    pub fn new(config: &'g GameConfig, state: GameState) -> Self {
        Self { config, state }
    }

    pub fn take_turn(&mut self, turn: Turn) -> Result<Option<Side>> {
        let num_boards = self.config.num_boards;

        // Process tech assignments
        let spells_bought = (turn.tech_assignment.num_spells() - 1) as i32;
        ensure!(spells_bought >= 0, "Cannot buy negative spells");
        
        let total_spell_cost = spells_bought * (num_boards as i32) * SPELL_COST;
        ensure!(self.state.money[self.state.side_to_move] >= total_spell_cost);
        self.state.money[self.state.side_to_move] -= total_spell_cost;

        self.state.tech_state.assign_techs(
            turn.tech_assignment, 
            self.state.side_to_move,
            &self.config.techline
        )?;

        // Process spell assignments
        ensure!(turn.spell_assignment.len() == self.state.boards.len(), 
            "Invalid spell_assignment length");
        for (_board_idx, _spell_idx) 
        in turn.spell_assignment.into_iter().enumerate() {
            // TODO: Handle spell assignment logic
        }

        // Process board actions for each board
        ensure!(turn.board_actions.len() == self.state.boards.len(),
            "Invalid board_actions length");

        for (board_idx, actions) 
        in turn.board_actions.into_iter().enumerate() {
            let board = &mut self.state.boards[board_idx];
            let map = &self.config.maps[board_idx];
            
            // Execute each action in sequence
            for action in actions {
                board.do_action(action, 
                    &mut self.state.money, 
                    &mut self.state.board_points,
                    &self.state.tech_state, 
                    map, 
                    self.state.side_to_move
                )?;
            }

            board.end_turn(
                &mut self.state.money, 
                map, 
                &mut self.state.board_points,
                self.state.side_to_move
            )?;
        }

        // Switch sides after the turn
        self.state.side_to_move = !self.state.side_to_move;

        Ok(self.winner())
    }

    pub fn winner(&self) -> Option<Side> {
        if self.state.board_points[!self.state.side_to_move] >= self.config.points_to_win {
            Some(!self.state.side_to_move)            
        } else if self.state.board_points[self.state.side_to_move] >= self.config.points_to_win {
            Some(self.state.side_to_move)
        } else {
            None
        }
    }

    /// Convert game state to FEN notation
    pub fn to_fen(&self) -> Result<String> {
        let mut fen = String::new();
        
        // Static configuration
        fen.push_str(&self.config.num_boards.to_string());
        fen.push(' ');
        
        fen.push_str(&self.config.maps
            .iter()
            .map(|m| m.to_index())
            .collect::<Result<Vec<_>>>()?
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(","));
        fen.push(' ');
        
        fen.push_str(&self.config.techline.techs.len().to_string());
        fen.push(' ');
        
        fen.push_str(&self.config.techline.techs
            .iter()
            .map(|t| t.to_index())
            .collect::<Result<Vec<_>>>()?
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(","));
        fen.push(' ');
        
        // Board states
        for (i, board) in self.state.boards.iter().enumerate() {
            if i > 0 {
                fen.push('|');
            }
            fen.push_str(&board.to_fen());
        }
        
        // Side to move
        fen.push(' ');
        fen.push_str(&self.state.side_to_move.to_index()?.to_string());
        
        // Tech status
        fen.push(' ');
        fen.push_str(&self.state.tech_state.to_fen()); 
        
        // Money
        fen.push(' ');
        fen.push_str(&format!("{}|{}", 
            self.state.money.get(Side::S0)?,
            self.state.money.get(Side::S1)?
        ));
        
        Ok(fen)
    }

    /// Parse game state from FEN notation
    pub fn parse_fen(fen: &str) -> Result<(GameConfig, GameState)> {
        let mut parts = fen.split_whitespace();
        
        // Parse static configuration
        let num_boards = parts.next()
            .ok_or_else(|| anyhow!("Missing number of boards"))?
            .parse::<usize>()?;
            
        let map_indices = parts.next()
            .ok_or_else(|| anyhow!("Missing map indices"))?
            .split(',')
            .map(|s| s.parse::<usize>())
            .collect::<Result<Vec<_>, _>>()?;
    
        let maps = map_indices
            .into_iter()
            .map(Map::from_index)
            .collect::<Result<Vec<_>>>()?;
            
        let num_techs = parts.next()
            .ok_or_else(|| anyhow!("Missing number of techs"))?
            .parse::<usize>()?;
            
        let tech_indices = parts.next()
            .ok_or_else(|| anyhow!("Missing tech indices"))?
            .split(',')
            .map(|s| s.parse::<usize>())
            .collect::<Result<Vec<_>, _>>()?;
    
        let techs = tech_indices
            .into_iter()
            .map(Tech::from_index)
            .collect::<Result<Vec<_>>>()?;
            
        if techs.len() != num_techs {
            bail!("Number of techs does not match the specified count");
        }
    
        let config = GameConfig {
            num_boards,
            points_to_win: todo!(),
            maps,
            techline: Techline { techs }, 
        };
        
        // Parse board states
        let board_states = parts.next()
            .ok_or_else(|| anyhow!("Missing board states"))?
            .split('|')
            .map(|s| Board::from_fen(s))
            .collect::<Result<Vec<_>>>()?;

        if board_states.len() != num_boards {
            bail!("Number of board states does not match the specified count");
        }
            
        // Parse side to move
        let side_to_move = Side::from_index(
            parts.next()
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or_else(|| anyhow!("Invalid side to move"))?
        )?;
            
        // Parse tech status
        let tech_status_str = parts.next()
            .ok_or_else(|| anyhow!("Missing tech status"))?;

        let tech_state = TechState::from_fen(tech_status_str, &config.techline)?;
        
        // Parse money
        let money_str = parts.next()
            .ok_or_else(|| anyhow!("Missing money"))?;
        let money: Vec<_> = money_str.split('|')
            .map(|s| s.parse::<i32>())
            .collect::<Result<_, _>>()?;
        let money = SideArray::new(money[0], money[1]);
        
        let state = GameState {
            side_to_move,
            boards: board_states,
            tech_state,
            money,
            board_points: todo!()
        };
        
        Ok((config, state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{loc::Loc, units::Unit};

    #[test]
    fn test_basic_fen_conversion() {
        let fen = "1 0 4 1,2,3,4 Z8i/0/0/0/0/0/0/0/0/0 0 LLLUUA|LLLLLA 10|5";
        let (config, state) = Game::parse_fen(fen).unwrap();

        assert_eq!(config.num_boards, 1);
        assert_eq!(config.maps.len(), 1);
        assert_eq!(config.techline.techs.len(), 4);
        assert_eq!(state.side_to_move, Side::S0);
        assert_eq!(*state.money.get(Side::S0).unwrap(), 10);
        assert_eq!(*state.money.get(Side::S1).unwrap(), 5);
    }

    #[test]
    fn test_empty_spaces_fen() {
        let fen = "2 0,1 4 1,2,3,4 0/0/0/0/0/0/0/0/0/0|0/0/0/0/0/0/0/0/0/0 1 LLLUUA|LLLLLA 10|5";
        let (config, state) = Game::parse_fen(fen).unwrap();

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
        assert!(Game::parse_fen("0 0 4 0,1,2,3 0/0/0/0/0/0/0/0/0/0 0 LLLUUA|LLLLLA 10|5").is_err());

        // Invalid tech status
        assert!(Game::parse_fen("1 0 4 0,1,2,3 0/0/0/0/0/0/0/0/0/0 0 LLLUUX|LLLLLA 10|5").is_err());

        // Invalid money format
        assert!(Game::parse_fen("1 0 4 0,1,2,3 0/0/0/0/0/0/0/0/0/0 0 LLLUUA|LLLLLA 10").is_err());
    }
}