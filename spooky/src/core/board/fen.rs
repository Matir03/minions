use anyhow::{anyhow, bail, ensure, Context, Result};

use crate::core::{
    loc::{Loc, GRID_LEN},
    map::Map,
    side::Side,
    units::Unit,
};

use super::{Board, Piece};

impl Board {
    /// Convert board state to FEN notation
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();
        for y in 0..GRID_LEN as i32 {
            let mut empty_squares = 0;
            for x in 0..GRID_LEN as i32 {
                let loc = Loc::new(x, y);
                if let Some(piece) = self.get_piece(&loc) {
                    if empty_squares > 0 {
                        fen.push_str(&empty_squares.to_string());
                        empty_squares = 0;
                    }
                    let mut c = piece.unit.to_fen_char();
                    if piece.side == Side::S1 {
                        c = c.to_ascii_lowercase();
                    }
                    fen.push(c);
                } else {
                    empty_squares += 1;
                }
            }
            if empty_squares > 0 {
                fen.push_str(&empty_squares.to_string());
            }
            if y < (GRID_LEN - 1) as i32 {
                fen.push('/');
            }
        }
        fen
    }

    /// Create a board from FEN notation
    pub fn from_fen(fen: &str, map: Map) -> Result<Self> {
        let mut board = Board::new(map);
        let mut y = 0;
        for row in fen.split('/') {
            let mut x = 0;
            while x < GRID_LEN as i32 {
                let c = row.chars().next().unwrap();
                if let Some(digit) = c.to_digit(10) {
                    if digit == 0 {
                        x += 10; 
                    } else {
                        x += digit as i32;
                    }
                } else {
                    let side = if c.is_uppercase() { Side::S0 } else { Side::S1 };
                    let unit = Unit::from_fen_char(c)
                        .with_context(|| format!("Invalid FEN char: {}", c))?;
                    let loc = Loc::new(x, y);
                    board.add_piece(Piece::new(unit, side, loc));
                    x += 1;
                }
            }
            y += 1;
        }
        Ok(board)
    }
}
