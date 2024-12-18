//! Game state and rules

use super::{
    board::{Board, Modifiers, Piece}, 
    map::{MapLabel, Loc}, 
    tech::{Tech, TechStatus, Techline},
    units::UnitLabel,
    side::{Side, SideArray},
    convert::{FromIndex, ToIndex},
};

use anyhow::{anyhow, Result};

/// Static game configuration
#[derive(Debug, Clone)]
pub struct GameConfig {
    pub num_boards: usize,
    pub maps: Vec<MapLabel>,
    pub techline: Techline,
}

/// State of a Minions game (excluding the static configuration)
#[derive(Debug, Clone)]
pub struct GameState {
    pub side_to_move: Side,
    pub boards: Vec<Board>,
    pub tech_status: SideArray<Vec<TechStatus>>,
    pub money: SideArray<i32>,
}

/// Represents the state of a Minions game
#[derive(Debug, Clone)]
pub struct Game<'g> {
    pub config: &'g GameConfig,
    pub state: GameState,
}

impl<'g> Game<'g> {
    /// Create a new game with the given configuration
    pub fn new(config: &'g GameConfig, state: GameState) -> Self {
        Self { config, state }
    }

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
        
        fen.push_str(&self.config.techline.num_techs.to_string());
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
            
            let mut empty_count = 0;
            
            for row in 0..10 {
                if row > 0 {
                    fen.push('/');
                }
                
                for col in 0..10 {
                    let loc = Loc { row, col };
                    if let Some(Piece { side, unit, .. }) = board.get_piece(loc) {
                        if empty_count > 0 {
                            if empty_count == 10 {
                                fen.push('0');
                            } else {
                                fen.push(char::from_digit(empty_count as u32, 10).unwrap());
                            }
                            empty_count = 0;
                        }
                        let unit_char = unit.to_fen_char();
                        match side {
                            Side::S0 => fen.push(unit_char.to_ascii_uppercase()),
                            Side::S1 => fen.push(unit_char.to_ascii_lowercase()),
                        }
                    } else {
                        empty_count += 1;
                    }
                }
                
                if empty_count > 0 {
                    if empty_count == 10 {
                        fen.push('0');
                    } else {
                        fen.push(char::from_digit(empty_count as u32, 10).unwrap());
                    }
                    empty_count = 0;
                }
            }
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
    
        // Create game config
        let config = GameConfig {
            num_boards,
            maps,
            techline: Techline { num_techs, techs }, 
        };
        
        // Parse board states
        let board_states = parts.next()
            .ok_or_else(|| anyhow!("Missing board states"))?
            .split('|')
            .map(|board_str| {
                let mut board = Board::new();
                let rows = board_str.split('/');
                for (row_idx, row) in rows.enumerate() {
                    let mut col = 0;
                    let mut chars = row.chars();
                    while let Some(c) = chars.next() {
                        if c.is_digit(10) {
                            let empty = if c == '0' { 10 } else { c.to_digit(10).unwrap() as usize };
                            col += empty;
                        } else {
                            let side = if c.is_ascii_uppercase() {
                                Side::S0
                            } else {
                                Side::S1
                            };
                            let unit = UnitLabel::from_fen_char(c.to_ascii_uppercase())
                                .ok_or_else(|| anyhow!("Invalid unit label"))?;
                            let piece = Piece {
                                loc: Loc { row: row_idx as i32, col: col as i32 },
                                side,
                                unit,
                                modifiers: Modifiers::default(),
                            };
                            board.add_piece(piece);
                            col += 1;
                        }
                    }
                    if col != 10 {
                        return Err(anyhow!("Invalid row length"));
                    }
                }
                Ok(board)
            })
            .collect::<Result<Vec<_>>>()?;
            
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