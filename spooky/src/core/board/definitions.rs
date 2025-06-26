use std::collections::HashMap;
use hashbag::HashBag;
use super::{bitboards::Bitboards, piece::Piece};

use crate::core::{
    loc::Loc,
    map::Map,
    side::{Side, SideArray},
    spells::Spell,
    units::Unit,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardState {
    FirstTurn,
    Normal,
    Reset0,
    Reset1,
    Reset2,
}

impl Default for BoardState {
    fn default() -> Self {
        Self::FirstTurn
    }
}

/// Represents a single Minions board
#[derive(Debug, Clone)]
pub struct Board<'a> {
    pub map: &'a Map,
    pub pieces: HashMap<Loc, Piece>,
    pub reinforcements: SideArray<HashBag<Unit>>,
    pub spells: SideArray<HashBag<Spell>>,
    pub winner: Option<Side>,
    pub state: BoardState,
    pub bitboards: Bitboards,
}

impl<'a> PartialEq for Board<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map
            && self.pieces == other.pieces
            && self.reinforcements == other.reinforcements
            && self.spells == other.spells
            && self.winner == other.winner
            && self.state == other.state
            && self.bitboards == other.bitboards
    }
}

impl<'a> Eq for Board<'a> {}

impl<'a> Board<'a> {
    pub const START_FEN: &'static str = "0/2ZZ6/1ZNZ6/1ZZ7/0/0/7zz1/6znz1/6zz2/0";
    pub const NECROMANCER_START_LOC: SideArray<Loc> = SideArray {
        values: [
            Loc { x: 2, y: 2 },
            Loc { x: 7, y: 7 },
        ]
    };
    pub const GRAVEYARDS_TO_WIN: i32 = 8;
}
