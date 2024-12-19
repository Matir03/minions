//! Game state and rules

use anyhow::{Result, anyhow, bail};
use crate::core::convert::{FromIndex, ToIndex};
use super::{
    board::Board,
    map::MapLabel,
    side::{Side, SideArray},
    tech::{Tech, TechStatus, Techline},
};

/// Static configuration for a Minions game
#[derive(Debug, Clone)]
pub struct GameConfig {
    pub num_boards: usize,
    pub maps: Vec<MapLabel>,
    pub techline: Techline,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            num_boards: 1,
            maps: vec![MapLabel::BlackenedShores],
            techline: Techline::default(),
        }
    }
}

/// State of a Minions game (excluding the static configuration)
#[derive(Debug, Clone)]
pub struct GameState {
    pub side_to_move: Side,
    pub boards: Vec<Board>,
    pub tech_status: SideArray<Vec<TechStatus>>,
    pub money: SideArray<i32>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            side_to_move: Side::S0,
            boards: vec![Board::default()],
            tech_status: SideArray::new(
                vec![TechStatus::Locked; Techline::NUM_TECHS],
                vec![TechStatus::Locked; Techline::NUM_TECHS]
            ),
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
        for (i, side_techs) in self.state.tech_status.iter().enumerate() {
            if i > 0 {
                fen.push('|');
            }
            for status in side_techs {
                fen.push(match status {
                    TechStatus::Locked => 'L',
                    TechStatus::Unlocked => 'U',
                    TechStatus::Acquired => 'A',
                });
            }
        }
        
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
            .map(MapLabel::from_index)
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
        let tech_status = tech_status_str.split('|')
            .map(|side_str| {
                side_str.chars()
                    .map(|c| match c {
                        'L' => Ok(TechStatus::Locked),
                        'U' => Ok(TechStatus::Unlocked),
                        'A' => Ok(TechStatus::Acquired),
                        _ => Err(anyhow!("Invalid tech status")),
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let tech_status = SideArray::new(
            tech_status[0].clone(),
            tech_status[1].clone(),
        );
        
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
            tech_status,
            money,
        };
        
        Ok((config, state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{map::Loc, units::UnitLabel};

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

    #[test]
    fn test_fen_with_modifiers() {
        let fen = "1 0 4 1,2,3,4 Z8i/0/0/0/0/0/0/0/0/0 0 LLLUUA|LLLLLA 10|5";
        let (_, state) = Game::parse_fen(fen).unwrap();

        let piece = state.boards[0].get_piece(Loc { row: 0, col: 0 }).unwrap();
        assert_eq!(piece.side, Side::S0);
        assert_eq!(piece.unit, UnitLabel::Zombie);
        assert!(!piece.modifiers.shielded);
    }

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