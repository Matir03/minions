use crate::core::{loc::Loc, side::Side, units::Unit};

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
    pub fn can_move(&self) -> bool {
        !self.moved && !self.exhausted && self.attacks_used == 0
    }

    pub fn can_attack(&self) -> bool {
        !self.exhausted
    }

    pub fn can_blink(&self) -> bool {
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

/// Represents a piece on the board
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub loc: Loc,
    pub side: Side,
    pub unit: Unit,
    pub modifiers: Modifiers,
    pub state: PieceState,
}

impl Piece {
    pub fn new(unit: Unit, side: Side, loc: Loc) -> Self {
        Self {
            unit,
            side,
            loc,
            modifiers: Modifiers::default(),
            state: PieceState::default(),
        }
    }
}
