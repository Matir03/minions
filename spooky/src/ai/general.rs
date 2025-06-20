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

#[derive(Clone)]
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
        let (money, config) = args;
        const P: f64 = 0.8;

        let side = self.side;
        let techline = &config.techline;
        let spell_cost = config.spell_cost();

        let assignment = {
            let mut acquire = vec![];
            let mut advance = 0;

            let max_spells = (money / spell_cost) as usize;
            let cur_index = self.tech_state.unlock_index[self.side];
            let max_reach = cur_index + max_spells;

            let dist = Bernoulli::new(P).unwrap();

            if self.tech_state.unlock_index[self.side] < techline.len() && max_spells > 0 {
                if cur_index < techline.len() {
                    advance = 1;
                    if self.tech_state.acquirable(cur_index) {
                        acquire.push(cur_index);
                    }
                }
            }

            if advance == 0 && acquire.is_empty() {
                for i in (0..max_reach).rev() {
                    if techline.techs.get(i).is_none() { continue; }
                    let can_tech = self.tech_state.acquirable(i);

                    if !can_tech {
                        continue;
                    }

                    if dist.sample(rng) {
                        advance = advance.max(i.saturating_sub(cur_index) + 1);
                        acquire.push(i);

                        let mut spells_left = max_spells.saturating_sub(advance);

                        for j in (0..i).rev() {
                            if spells_left == 0 {
                                break;
                            }
                            if techline.techs.get(j).is_none() { continue; }
                            let can_tech_j = self.tech_state.acquirable(j);

                            if !can_tech_j {
                                continue;
                            }

                            if dist.sample(rng) {
                                acquire.push(j);
                                spells_left = spells_left.saturating_sub(1);
                            }
                        }
                        break;
                    }
                }
            }

            if advance == 0 && acquire.is_empty() {
                TechAssignment::new(0, vec![])
            } else {
                TechAssignment::new(advance, acquire)
            }
        };

        let num_spells = assignment.num_spells();
        let mut new_delta_money = SideArray::new(0, 0);
        new_delta_money[side] = -( (num_spells as i32) * spell_cost );

        let mut new_tech_state = self.tech_state.clone();
        if new_tech_state.assign_techs(assignment.clone(), side, techline).is_err() {
            return (TechAssignment::new(0, vec![]), Self::new(self.side, self.tech_state.clone(), SideArray::new(0,0)));
        }

        let new_node_state = Self::new(self.side, new_tech_state, new_delta_money);

        (assignment, new_node_state)
    }
}

pub type GeneralNode<'a> = MCTSNode<'a, GeneralNodeState, TechAssignment>;
pub type GeneralNodeRef<'a> = &'a RefCell<GeneralNode<'a>>;