use bumpalo::{Bump, collections::Vec};
use std::cell::RefCell;
use std::vec::Vec as StdVec;

use crate::core::{
    Side,
    side::SideArray,
    GameConfig,
    tech::{TechAssignment, TechState},
};

use rand::{distributions::Bernoulli, prelude::*};

use super::{
    mcts::{NodeState, NodeStats, MCTSNode},
    eval::Eval,
    search::SearchArgs,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneralNodeState {
    pub side: Side,
    pub tech_state: TechState,
    pub delta_money: SideArray<i32>,
}

impl GeneralNodeState {
    pub fn new(side: Side, tech_state: TechState, delta_money: SideArray<i32>) -> Self {
        Self {
            side,
            tech_state,
            delta_money,
        }
    }
}

impl NodeState<TechAssignment> for GeneralNodeState {
    type Args = (i32, GameConfig);

    fn propose_move(&self, rng: &mut impl Rng, args: &Self::Args) -> (TechAssignment, Self) {
        let (_money, config) = args;
        let spell_cost = config.spell_cost();
        let techline = &config.techline;
        let side = self.side;

        let mut possible_assignments = StdVec::new();

        // Option 1: Advance by 1
        if self.tech_state.unlock_index[side] < techline.len() {
            possible_assignments.push(TechAssignment::new(1, vec![]));
        }

        // Option 2: Acquire one
        for i in 0..self.tech_state.unlock_index[side] {
            if self.tech_state.acquirable(i, side) {
                possible_assignments.push(TechAssignment::new(0, vec![i]));
            }
        }

        let assignment = if possible_assignments.is_empty() {
            TechAssignment::new(0, vec![])
        } else {
            possible_assignments.choose(rng).unwrap().clone()
        };

        let num_spells = assignment.num_spells();
        let spells_bought = num_spells - 1;
        let mut new_delta_money = SideArray::new(0, 0);
        if spells_bought > 0 {
            new_delta_money[side] = -(spells_bought * spell_cost);
        }

        let mut new_tech_state = self.tech_state.clone();
        if new_tech_state.assign_techs(assignment.clone(), side, techline).is_err() {
            return (TechAssignment::new(0, vec![]), Self::new(self.side, self.tech_state.clone(), SideArray::new(0,0)));
        }

        let new_node_state = Self::new(!self.side, new_tech_state, new_delta_money);

        (assignment, new_node_state)
    }
}

pub type GeneralNode<'a> = MCTSNode<'a, GeneralNodeState, TechAssignment>;
pub type GeneralNodeRef<'a> = &'a RefCell<GeneralNode<'a>>;