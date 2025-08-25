use super::{bitboards::Bitboards, piece::Piece};
use hashbag::HashBag;
use std::collections::HashMap;

use crate::core::{
    loc::Loc,
    map::Map,
    side::{Side, SideArray},
    spells::Spell,
    units::Unit,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Setup,
    Attack,
    Spawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardState {
    FirstTurn,
    Normal,
    Reset1,
    Reset2,
}

impl Default for BoardState {
    fn default() -> Self {
        Self::FirstTurn
    }
}

impl BoardState {
    pub fn phases(&self) -> &'static [Phase] {
        match self {
            BoardState::FirstTurn => &[Phase::Attack],
            BoardState::Normal => &[Phase::Attack, Phase::Spawn],
            BoardState::Reset1 => &[Phase::Setup, Phase::Attack], // just ended, going first
            BoardState::Reset2 => &[Phase::Setup, Phase::Attack, Phase::Spawn], // just ended, going second
        }
    }
}

/// Represents a single Minions board
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board<'a> {
    pub map: &'a Map,
    pub pieces: HashMap<Loc, Piece>,
    pub reinforcements: SideArray<HashBag<Unit>>,
    pub spells: SideArray<HashBag<Spell>>,
    pub winner: Option<Side>,
    pub state: BoardState,
    pub bitboards: Bitboards,
}

impl<'a> Board<'a> {
    pub const START_FEN: &'static str = "f|I|i|||0/2ZZ6/1ZNZ6/1ZZ7/0/0/7zz1/6znz1/6zz2/0";
    pub const NECROMANCER_START_LOC: SideArray<Loc> = SideArray {
        values: [Loc { x: 2, y: 2 }, Loc { x: 7, y: 7 }],
    };
    pub const GRAVEYARDS_TO_WIN: i32 = 8;
}
