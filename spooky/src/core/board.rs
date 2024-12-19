//! Board representation and rules

use std::collections::{HashMap, VecDeque};
use anyhow::{Result, bail};
use super::{
    side::{Side, SideArray},
    units::UnitLabel,
    map::Loc,
};

/// Status modifiers that can be applied to pieces
#[derive(Debug, Clone, Default)]
pub struct Modifiers {
    pub shielded: bool,
    pub frozen: bool,
    pub shackled: bool,
}

/// Represents a piece on the board
#[derive(Debug, Clone)]
pub struct Piece {
    pub loc: Loc,
    pub side: Side,
    pub unit: UnitLabel,
    pub modifiers: Modifiers,
}

/// Types of spells that can be cast
#[derive(Debug, Clone)]
pub enum Spell {
    Shield,
    Summon {
        loc: Loc,
        unit: UnitLabel,
    },
    Reposition,
    // Add other spell types
}

/// Represents a single Minions board
#[derive(Debug, Clone)]
pub struct Board {
    pub pieces: HashMap<Loc, Piece>,
    pub reinforcements: SideArray<VecDeque<UnitLabel>>,
    pub spells: SideArray<Vec<Spell>>,
}

impl Board {
    /// Create a new empty board
    pub fn new() -> Self {
        Self {
            pieces: HashMap::new(),
            reinforcements: SideArray::new(VecDeque::new(), VecDeque::new()),
            spells: SideArray::new(Vec::new(), Vec::new()),
        }
    }

    pub fn get_piece(&self, loc: Loc) -> Option<&Piece> {
        self.pieces.get(&loc)
    }

    /// Add a piece to the board
    pub fn add_piece(&mut self, piece: Piece) {
        self.pieces.insert(piece.loc, piece);
    }

    /// Remove a piece from the board
    pub fn remove_piece(&mut self, loc: Loc) -> Option<Piece> {
        self.pieces.remove(&loc)
    }

    /// Convert board state to FEN notation
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();
        let mut empty_count = 0;
        
        for row in 0..10 {
            if row > 0 {
                fen.push('/');
            }
            
            for col in 0..10 {
                let loc = Loc { row, col };
                if let Some(piece) = self.get_piece(loc) {
                    if empty_count > 0 {
                        if empty_count == 10 {
                            fen.push('0');
                        } else {
                            fen.push(char::from_digit(empty_count as u32, 10).unwrap());
                        }
                        empty_count = 0;
                    }
                    let mut c = piece.unit.to_fen_char();
                    if piece.side == Side::S1 {
                        c = c.to_ascii_lowercase();
                    }
                    fen.push(c);
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
        
        fen
    }

    /// Create a board from FEN notation
    pub fn from_fen(fen: &str) -> Result<Self> {
        let mut board = Board::new();
        let mut row = 0;
        let mut col = 0;

        for c in fen.chars() {
            match c {
                '/' => {
                    if col != 10 {
                        bail!("Invalid FEN: wrong number of squares in row {}", row);
                    }
                    row += 1;
                    col = 0;
                }
                '0'..='9' => {
                    let empty = if c == '0' { 10 } else { c.to_digit(10).unwrap() as usize };
                    col += empty;
                }
                'A'..='Z' | 'a'..='z' => {
                    if col >= 10 || row >= 10 {
                        bail!("Invalid FEN: piece position out of bounds");
                    }

                    let side = if c.is_uppercase() { Side::S0 } else { Side::S1 };
                    if let Some(unit) = UnitLabel::from_fen_char(c.to_ascii_uppercase()) {
                        board.add_piece(Piece {
                            loc: Loc { 
                                row: row as i32, 
                                col: col as i32 
                            },
                            side,
                            unit,
                            modifiers: Modifiers::default(),
                        });
                        col += 1;
                    } else {
                        bail!("Invalid FEN: unknown piece '{}'", c);
                    }
                }
                _ => bail!("Invalid FEN character: '{}'", c),
            }
        }

        if row != 9 || col != 10 {
            bail!("Invalid FEN: wrong number of rows or columns");
        }

        Ok(board)
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_pieces() {
        let mut board = Board::new();
        let piece = Piece {
            loc: Loc { row: 0, col: 0 },
            side: Side::S0,
            unit: UnitLabel::Zombie,
            modifiers: Modifiers::default(),
        };
        board.add_piece(piece);

        assert!(board.get_piece(Loc { row: 0, col: 0 }).is_some());
        assert!(board.get_piece(Loc { row: 0, col: 1 }).is_none());
    }

    #[test]
    fn test_board_fen_conversion() {
        let mut board = Board::new();
        
        // Add a zombie at (0,0) for Side 0
        board.add_piece(Piece {
            loc: Loc { row: 0, col: 0 },
            side: Side::S0,
            unit: UnitLabel::Zombie,
            modifiers: Modifiers::default(),
        });

        // Add an initiate at (0,9) for Side 1
        board.add_piece(Piece {
            loc: Loc { row: 0, col: 9 },
            side: Side::S1,
            unit: UnitLabel::Initiate,
            modifiers: Modifiers::default(),
        });

        assert_eq!(board.to_fen(), "Z8i/0/0/0/0/0/0/0/0/0");

        let board2 = Board::from_fen(&board.to_fen()).unwrap();
        assert_eq!(board2.to_fen(), board.to_fen());
    }

    #[test]
    fn test_fen_empty_board() {
        let board = Board::new();
        assert_eq!(board.to_fen(), "0/0/0/0/0/0/0/0/0/0");

        let board2 = Board::from_fen("0/0/0/0/0/0/0/0/0/0").unwrap();
        assert_eq!(board2.to_fen(), "0/0/0/0/0/0/0/0/0/0");
    }

    #[test]
    fn test_invalid_fen() {
        // Test invalid piece character
        assert!(Board::from_fen("X9/0/0/0/0/0/0/0/0/0").is_err());

        // Test wrong number of rows
        assert!(Board::from_fen("0/0/0/0/0/0/0/0/0").is_err());

        // Test wrong number of squares in row
        assert!(Board::from_fen("Z8/0/0/0/0/0/0/0/0/0").is_err());
    }
}
