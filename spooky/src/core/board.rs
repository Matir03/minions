//! Board representation and rules

use std::collections::VecDeque;
use super::{
    side::{Side, SideArray},
    units::UnitLabel,
    map::Loc,
    game,
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
    Reposition,
    // Add other spell types
}

/// Represents a single Minions board
#[derive(Debug, Clone)]
pub struct Board {
    pub pieces: Vec<Piece>,
    pub reinforcements: SideArray<VecDeque<UnitLabel>>,
    pub spells: SideArray<Vec<Spell>>,
}

impl Board {
    /// Create a new empty board
    pub fn new() -> Self {
        Self {
            pieces: Vec::new(),
            reinforcements: SideArray::new(VecDeque::new(), VecDeque::new()),
            spells: SideArray::new(Vec::new(), Vec::new()),
        }
    }

    pub fn get_piece(&self, loc: Loc) -> Option<&Piece> {
        self.pieces.iter().find(|p| p.loc == loc)
    }

    /// Add a piece to the board
    pub fn add_piece(&mut self, piece: Piece) {
        self.pieces.push(piece);
    }

    /// Remove a piece from the board
    pub fn remove_piece(&mut self, loc: Loc) -> Option<Piece> {
        if let Some(index) = self.pieces.iter().position(|p| p.loc == loc) {
            Some(self.pieces.remove(index))
        } else {
            None
        }
    }
}
