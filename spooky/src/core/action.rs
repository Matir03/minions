//! Game actions and moves

use super::{
    map::Loc,
    units::UnitLabel,
    board::Spell,
};

/// Parameters for spell casting
#[derive(Debug, Clone)]
pub struct CastParams {
    pub target_loc: Loc,
    // Add other spell parameters as needed
}

/// Represents a legal move in the game
#[derive(Debug, Clone)]
pub enum BoardAction {
    Move {
        from_sq: Loc,
        to_sq: Loc,
    },
    Attack {
        attacker_sq: Loc,
        target_sq: Loc,
    },
    Blink {
        blink_sq: Loc,
    },
    Spawn {
        spawn_sq: Loc,
        unit: UnitLabel,
    },
    Cast {
        spell: Spell,
        params: CastParams,
    },
    Discard {
        spell: Spell,
    },
    EndPhase,
}

/// Represents a complete turn in the game
#[derive(Debug, Clone)]
pub struct Move {
    pub num_spells_bought: usize,
    pub board_spells: Vec<usize>,  // board -> spell index
    pub tech_spells: Vec<usize>,   // techline indices
    pub board_actions: Vec<Vec<BoardAction>>,
}

impl Move {
    /// Create a new turn with the given number of boards
    pub fn new(num_boards: usize) -> Self {
        Self {
            num_spells_bought: 0,
            board_spells: vec![0; num_boards],
            tech_spells: Vec::new(),
            board_actions: vec![Vec::new(); num_boards],
        }
    }

    /// Add an action to a specific board
    pub fn add_action(&mut self, board_index: usize, action: BoardAction) {
        if board_index < self.board_actions.len() {
            self.board_actions[board_index].push(action);
        }
    }
}

#[cfg(test)]
mod tests { }
