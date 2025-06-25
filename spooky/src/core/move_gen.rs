//! Move generation for the spooky engine.

use crate::core::board::Board;
use crate::core::board::actions::AttackAction;
use crate::core::side::Side;

/// Generates all possible attack actions for a given side on a given board.
pub fn generate_attack_actions(board: &Board, side: Side) -> Vec<AttackAction> {
    let mut actions = Vec::new();

    // TODO: Implement move generation logic here.

    actions
}
