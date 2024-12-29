use super::{
    attack::{AttackNode, AttackNodeRef}, eval::Eval, mcts::{MCTSNode, NodeStats, Stage}, search::SearchArgs
};
use crate::core::{Board, SideArray};

use std::cell::RefCell;

use rand::prelude::*;

use bumpalo::{Bump, collections::Vec};

pub struct SpawnNode<'a> {
    pub stats: NodeStats,
    pub board: Board,
    pub children: Vec<'a, AttackNodeRef<'a>>,
}

impl <'a> SpawnNode<'a> {
    pub fn new(board: Board, arena: &'a Bump) -> Self {
        Self {
            stats: NodeStats::new(),
            board,
            children: Vec::new_in(arena),
        }
    }

    pub fn update(&mut self, eval: &Eval) {
        self.stats.update(eval);
    }
}

pub type SpawnNodeRef<'a> = &'a RefCell<SpawnNode<'a>>;

impl<'a> MCTSNode<'a> for SpawnNode<'a> {
    type Child = AttackNode<'a>;
    type Etc = i32;

    fn stats(&self) -> &NodeStats {
        &self.stats
    }

    fn children(&self) -> &Vec<'a, AttackNodeRef<'a>> {
        &self.children
    }

    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, blotto: i32) -> (bool, usize) {
        todo!()
    }

    fn update(&mut self, eval: &Eval) {
        self.stats.update(eval);
    }
}