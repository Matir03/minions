use anyhow::{anyhow, bail, ensure, Context, Result};


use crate::core::{
    loc::{Loc, GRID_LEN},
    map::Map,
    side::Side,
    units::Unit,
    board::{Modifiers, PieceState, RefCell},
};

use super::{Board, Piece};

impl<'a> Board<'a> {
    /// Convert board state to FEN notation
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();
        let mut empty_count = 0;
        
        for y in 0..10 {
            if y > 0 {
                fen.push('/');
            }
            
            for x in 0..10 {
                let loc = Loc { y, x };
                if let Ok(piece) = self.get_piece(&loc) {
                    if empty_count > 0 {
                        if empty_count == 10 {
                            fen.push('0');
                        } else {
                            fen.push(char::from_digit(empty_count as u32, 10).unwrap());
                        }
                        empty_count = 0;
                    }
                    let mut c = piece.unit.to_fen_char();
                    if piece.side == Side::Blue {
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
        pub fn from_fen(fen: &str, map: &'a Map) -> Result<Self> {
        let mut board = Self::new(map);
        let parts: Vec<&str> = fen.split('/').collect();

        ensure!(parts.len() == 10, "Invalid FEN: must have 10 rows");

        for (y, row_str) in parts.iter().enumerate() {
            let mut x = 0;
            if *row_str == "0" {
                x = 10;
            } else {
                for c in row_str.chars() {
                    if let Some(digit) = c.to_digit(10) {
                        ensure!(digit != 0, "FEN digit cannot be 0 unless it's the only char in the row");
                        x += digit as i32;
                    } else {
                        let side = if c.is_uppercase() { Side::Yellow } else { Side::Blue };
                        let unit_char = c.to_ascii_lowercase();
                        let unit = Unit::from_fen_char(unit_char)
                            .ok_or_else(|| anyhow!("Invalid unit char: {}", unit_char))?;

                        let loc = Loc::new(x, y as i32);
                        board.add_piece(Piece {
                            loc,
                            side,
                            unit,
                            modifiers: Modifiers::default(),
                            state: PieceState::default(),
                        });
                        x += 1;
                    }
                }
            }
            ensure!(x == 10, "Invalid FEN: row {} does not sum to 10, got {}", y, x);
        }

        Ok(board)
    }
}
