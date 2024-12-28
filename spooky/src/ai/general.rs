use bumpalo::{Bump, collections::Vec};
use std::cell::RefCell;

use crate::core::{
    Side,
    side::SideArray,
    GameConfig,
    tech::{TechAssignment, TechState},
};

use super::{
    mcts::NodeStats,
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

    pub fn explore(&mut self, args: &SearchArgs<'a>) -> &'a mut GeneralNode<'a> {
        todo!()
    }

    pub fn eval(&self) -> Eval {
        todo!()
    }

    pub fn best_assignment(&self) -> TechAssignment {
        todo!()
    }
}