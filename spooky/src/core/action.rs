//! Game actions and moves

use super::{
    loc::Loc,
    units::Unit,
    spells::{Spell, SpellCast},
    tech::TechAssignment
};

/// Represents a legal move in the game
#[derive(Debug, Clone)]
pub enum BoardAction {
    Move {
        from_loc: Loc,
        to_loc: Loc,
    },
    MoveCyclic {
        locs: Vec<Loc>,
    },
    Attack {
        attacker_loc: Loc,
        target_loc: Loc,
    },
    Blink {
        blink_loc: Loc,
    },
    Buy {
        unit: Unit,
    },
    Spawn {
        spawn_loc: Loc,
        unit: Unit,
    },
    Cast {
        spell_cast: SpellCast,
    },
    Discard {
        spell: Spell,
    },
    EndPhase,
}

/// Represents a complete turn in the game
#[derive(Debug, Clone)]
pub struct Turn {
    // pub num_spells_bought: usize,
    pub spell_assignment: Vec<usize>,  // board -> spell index
    pub tech_assignment: TechAssignment,
    pub board_actions: Vec<Vec<BoardAction>>,
}

impl Turn {
    /// Create a new turn with the given number of boards
    pub fn new(num_boards: usize) -> Self {
        Self {
            // num_spells_bought: 0,
            spell_assignment: vec![0; num_boards],
            tech_assignment: TechAssignment::default(),
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
