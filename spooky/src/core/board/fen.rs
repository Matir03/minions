use anyhow::{anyhow, bail, ensure, Context, Result};
use hashbag::HashBag;

use crate::core::{
    board::{BoardState, Modifiers, PieceState, RefCell},
    loc::{Loc, GRID_LEN},
    map::Map,
    side::Side,
    units::Unit,
    Spell,
};

use super::{Board, Piece};

impl<'a> Board<'a> {
    /// Convert board state to FEN notation
    pub fn to_fen(&self) -> String {
        // Build the complete FEN string with all components separated by |
        [
            // Component 1: Board state
            self.state.to_fen(),
            // Component 2: Yellow reinforcements
            reinforcements_to_fen(&self.reinforcements[Side::Yellow], Side::Yellow),
            // Component 3: Blue reinforcements
            reinforcements_to_fen(&self.reinforcements[Side::Blue], Side::Blue),
            // Component 4: Yellow spells
            spells_to_fen(&self.spells[Side::Yellow]),
            // Component 5: Blue spells
            spells_to_fen(&self.spells[Side::Blue]),
            // Component 6: Board position
            self.position_to_fen(),
        ]
        .join("|")
    }

    /// Convert just the board position to FEN notation
    fn position_to_fen(&self) -> String {
        let mut fen = String::new();
        let mut empty_count = 0;

        for y in 0..10 {
            if y > 0 {
                fen.push('/');
            }

            for x in 0..10 {
                let loc = Loc { x, y };
                if let Ok(piece) = self.get_piece(&loc) {
                    if empty_count > 0 {
                        if empty_count == 10 {
                            fen.push('0');
                        } else {
                            fen.push(char::from_digit(empty_count as u32, 10).unwrap());
                        }
                        empty_count = 0;
                    }
                    let c = piece.unit.to_fen_char_side(piece.side);
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

        let components = fen.split("|").collect::<Vec<&str>>();

        let position = if components.len() == 6 {
            // New format: state|yellow_reinforcements|blue_reinforcements|yellow_spells|blue_spells|position
            let (
                state,
                yellow_reinforcements,
                blue_reinforcements,
                yellow_spells,
                blue_spells,
                position,
            ) = (
                components[0],
                components[1],
                components[2],
                components[3],
                components[4],
                components[5],
            );

            board.state = BoardState::from_fen(state)?;

            board.reinforcements[Side::Yellow] = reinforcements_from_fen(yellow_reinforcements)?;
            board.reinforcements[Side::Blue] = reinforcements_from_fen(blue_reinforcements)?;

            board.spells[Side::Yellow] = spells_from_fen(yellow_spells)?;
            board.spells[Side::Blue] = spells_from_fen(blue_spells)?;

            position
        } else {
            bail!(
                "Invalid FEN format: expected 6 components separated by '|', got {}",
                components.len()
            );
        };

        let parts: Vec<&str> = position.split('/').collect();

        ensure!(parts.len() == 10, "Invalid position FEN: must have 10 rows");

        for (y, row_str) in parts.iter().enumerate() {
            let mut x = 0;
            if *row_str == "0" {
                x = 10;
            } else {
                for c in row_str.chars() {
                    if let Some(digit) = c.to_digit(10) {
                        ensure!(
                            digit != 0,
                            "FEN digit cannot be 0 unless it's the only char in the row"
                        );
                        x += digit as i32;
                    } else {
                        let side = if c.is_uppercase() {
                            Side::Yellow
                        } else {
                            Side::Blue
                        };
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
            ensure!(
                x == 10,
                "Invalid FEN: row {} does not sum to 10, got {}",
                y,
                x
            );
        }

        Ok(board)
    }
}

impl BoardState {
    #[allow(clippy::wrong_self_convention)]
    fn to_fen(&self) -> String {
        match self {
            Self::FirstTurn => "f".to_string(),
            Self::Normal => "n".to_string(),
            Self::Reset1 => "1".to_string(),
            Self::Reset2 => "2".to_string(),
        }
    }

    fn from_fen(fen: &str) -> Result<Self> {
        match fen {
            "f" => Ok(Self::FirstTurn),
            "n" => Ok(Self::Normal),
            "1" => Ok(Self::Reset1),
            "2" => Ok(Self::Reset2),
            _ => Err(anyhow!("Invalid board state: {}", fen)),
        }
    }
}

fn reinforcements_from_fen(fen: &str) -> Result<HashBag<Unit>> {
    let mut reinforcements = HashBag::new();
    for char in fen.chars() {
        reinforcements
            .insert(Unit::from_fen_char(char).context("Invalid unit char in reinforcements")?);
    }
    Ok(reinforcements)
}

fn spells_from_fen(fen: &str) -> Result<HashBag<Spell>> {
    if fen.is_empty() {
        return Ok(HashBag::new());
    }
    let mut spells = HashBag::new();
    for spell in fen.split(",") {
        spells.insert(spell.parse()?);
    }
    Ok(spells)
}

fn reinforcements_to_fen(reinforcements: &HashBag<Unit>, side: Side) -> String {
    let mut fen = String::new();
    for (unit, count) in reinforcements.set_iter() {
        for _ in 0..count {
            fen.push(unit.to_fen_char_side(side));
        }
    }
    let mut chars: Vec<char> = fen.chars().collect();
    chars.sort();
    chars.into_iter().collect()
}

fn spells_to_fen(spells: &HashBag<Spell>) -> String {
    // let mut fens = Vec::new();
    // for (spell, count) in spells.set_iter() {
    //     for _ in 0..count {
    //         fens.push(spell.to_string());
    //     }
    // }
    // fens.join(",")
    "".to_string()
}
