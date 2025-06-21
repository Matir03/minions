use super::{eval::Eval, mcts::{MCTSNode, NodeStats}, search::SearchArgs};
use crate::core::{Board, SideArray, Side, units::Unit, action::BoardAction};

use std::cell::RefCell;

use rand::prelude::*;

use std::vec::Vec;


pub struct SpawnStage;

impl SpawnStage {
    /// Generate spawn actions for a board given the allocated money.
    pub fn generate_spawns(board: &Board, side: Side, money: i32, rng: &mut impl Rng) -> Vec<BoardAction> {
        vec![BoardAction::EndPhase]
    }
}