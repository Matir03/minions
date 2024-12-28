use super::{
    attack::{AttackNode, AttackNodeRef}, 
    mcts::{NodeStats, Stage},
};
use crate::core::Board;

use std::cell::RefCell;

use bumpalo::{Bump, collections::Vec};

pub struct SpawnNode<'a> {
    pub stats: NodeStats,
    pub board: Board,
    pub children: Vec<'a, AttackNodeRef<'a>>,
}

pub type SpawnNodeRef<'a> = &'a RefCell<SpawnNode<'a>>;