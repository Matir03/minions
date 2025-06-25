use std::{cell::RefCell, collections::HashMap, sync::Arc};
use hashbag::HashBag;
use super::bitboards::Bitboards;

use crate::core::{
    loc::Loc,
    map::Map,
    side::{Side, SideArray},
    spells::Spell,
    units::Unit,
};

/// Status modifiers that can be applied to pieces
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Modifiers {
    // pub shielded: bool,
    // pub frozen: bool,
    // pub shackled: bool,
    // TODO: Add other modifiers
}

#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
pub struct PieceState {
    pub moved: bool,
    pub attacks_used: i32,
    pub damage_taken: i32,
    pub exhausted: bool,
}

impl PieceState {
    pub fn can_act(&self) -> bool {
        !self.exhausted
    }

    pub fn spawned() -> Self {
        Self {
            moved: false,
            attacks_used: 0,
            damage_taken: 0,
            exhausted: true,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

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

/// Represents a piece on the board
#[derive(Debug, Clone)]
pub struct Piece {
    pub loc: Loc,
    pub side: Side,
    pub unit: Unit,
    pub modifiers: Modifiers,
    pub state: RefCell<PieceState>,
}

impl PartialEq for Piece {
    fn eq(&self, other: &Self) -> bool {
        self.loc == other.loc
            && self.side == other.side
            && self.unit == other.unit
            && self.modifiers == other.modifiers
            && *self.state.borrow() == *other.state.borrow()
    }
}

impl Eq for Piece {}

impl Piece {
    pub fn new(unit: Unit, side: Side, loc: Loc) -> Self {
        Self {
            unit,
            side,
            loc,
            modifiers: Modifiers::default(),
            state: RefCell::new(PieceState::default()),
        }
    }
}

/// Represents a single Minions board
#[derive(Debug, Clone)]
pub struct Board {
    pub map: Arc<Map>,
    pub pieces: HashMap<Loc, Piece>,
    pub reinforcements: SideArray<HashBag<Unit>>,
    pub spells: SideArray<HashBag<Spell>>,
    pub winner: Option<Side>,
    pub state: BoardState,
    pub bitboards: Bitboards,
}

impl PartialEq for Board {
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

impl Eq for Board {}

impl Board {
    pub const START_FEN: &str = "0/2ZZ6/1ZNZ6/1ZZ7/0/0/7zz1/6znz1/6zz2/0";
    pub const NECROMANCER_START_LOC: SideArray<Loc> = SideArray {
        values: [
            Loc { x: 2, y: 2 },
            Loc { x: 7, y: 7 },
        ]
    };
    pub const GRAVEYARDS_TO_WIN: i32 = 8;
}
