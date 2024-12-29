use bumpalo::{Bump, collections::Vec};
use std::cell::RefCell;

use crate::core::{
    Side,
    side::SideArray,
    GameConfig,
    tech::{TechAssignment, TechState},
};

use rand::prelude::*;

use super::{
    mcts::{NodeStats, MCTSNode},
    eval::Eval,
    search::SearchArgs,
};

pub struct GeneralNode<'a> {
    pub stats: NodeStats,
    pub tech_state: TechState,
    pub delta_money: SideArray<i32>,
    pub children: Vec<'a, GeneralNodeRef<'a>>,
}

pub type GeneralNodeRef<'a> = &'a RefCell<GeneralNode<'a>>;

impl <'a> GeneralNode<'a> {
    pub fn new(tech_state: TechState, delta_money: SideArray<i32>, arena: &'a Bump) -> Self {
        Self {
            stats: NodeStats::new(),
            tech_state,
            delta_money,
            children: Vec::new_in(arena),
        }
    }

    pub fn best_assignment(&self) -> TechAssignment {
        todo!()
    }
}

impl<'a> MCTSNode<'a> for GeneralNode<'a> {
    type Child = GeneralNode<'a>;
    type Etc = ();

    fn stats(&self) -> &NodeStats {
        &self.stats
    }

    fn children(&self) -> &Vec<'a, GeneralNodeRef<'a>> {
        &self.children
    }

    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, _etc: ()) -> (bool, usize) {
        todo!()
    }

    fn update(&mut self, eval: &Eval) {
        self.stats.update(eval);
    }
}