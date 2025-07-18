use bumpalo::{collections::Vec, Bump};
use std::cell::RefCell;
use std::vec::Vec as StdVec;

use crate::core::{
    convert::ToIndex,
    side::SideArray,
    tech::{TechAssignment, TechState, NUM_TECHS},
    GameConfig, Side, Unit,
};

use rand::{distributions::Bernoulli, prelude::*};

use super::{
    eval::Eval,
    mcts::{MCTSNode, NodeState, NodeStats},
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

    fn propose_move(&self, _rng: &mut impl Rng, args: &Self::Args) -> (TechAssignment, Self) {
        let (money, config) = args;
        let spell_cost = config.spell_cost();
        let num_spells = money / spell_cost + 1;
        let techline = &config.techline;
        let side = self.side;
        let opponent = !side;

        let our_techs = &self.tech_state.acquired_techs[side];
        let enemy_techs_set = &self.tech_state.acquired_techs[opponent];

        let mut enemy_techs: StdVec<_> = enemy_techs_set.iter().copied().collect();
        enemy_techs.sort();

        let get_counters = |i: usize| -> StdVec<usize> {
            let mut counters = StdVec::new();
            if i + 1 < NUM_TECHS {
                counters.push(i + 1);
            }
            if i + 2 < NUM_TECHS {
                counters.push(i + 2);
            }
            if i >= 4 {
                counters.push(i - 3);
            }
            counters
        };

        let mut highest_uncountered_enemy_tech = None;
        for &enemy_tech in enemy_techs.iter().rev() {
            let enemy_tech_idx = enemy_tech.to_index().unwrap();
            let counters = get_counters(enemy_tech_idx);
            let is_countered = counters.iter().any(|&c| {
                let tech = techline.techs[c];
                our_techs.contains(&tech)
            });
            if !is_countered {
                highest_uncountered_enemy_tech = Some(enemy_tech);
                break;
            }
        }

        let mut target_tech_idx = None;

        if let Some(enemy_tech) = highest_uncountered_enemy_tech {
            let enemy_idx = enemy_tech.to_index().unwrap();
            // Prioritize countering the highest uncountered enemy unit.
            let counters = get_counters(enemy_idx);
            target_tech_idx = counters
                .into_iter()
                .filter(|&idx| self.tech_state.acquirable(idx, side, num_spells))
                .max(); // Tech to the highest available counter
        } else {
            // If all enemy units are countered, press the advantage.
            if let Some(&our_highest_tech) = our_techs.iter().max() {
                let our_highest_tech_idx = our_highest_tech.to_index().unwrap();
                let n_plus_3 = our_highest_tech_idx + 3;
                let n_plus_5 = our_highest_tech_idx + 5;

                if self.tech_state.acquirable(n_plus_3, side, num_spells) {
                    target_tech_idx = Some(n_plus_3);
                } else if self.tech_state.acquirable(n_plus_5, side, num_spells) {
                    target_tech_idx = Some(n_plus_5);
                }
            }
            if target_tech_idx.is_none() {
                // Fallback: tech to the highest available unit.
                target_tech_idx = (0..self.tech_state.unlock_index[side])
                    .filter(|&i| self.tech_state.acquirable(i, side, num_spells))
                    .max();
            }
        }

        let assignment = if let Some(idx) = target_tech_idx {
            let num_advance = (idx as i32 - self.tech_state.unlock_index[side] as i32 + 1).max(0);
            TechAssignment::new(num_advance as usize, vec![idx])
        } else {
            // If no specific target, just march if possible.
            if self.tech_state.unlock_index[side] < techline.len() {
                TechAssignment::new(1, vec![])
            } else {
                TechAssignment::new(0, vec![]) // Pass
            }
        };

        let assignment_spells = assignment.num_spells();
        let spells_bought = if assignment_spells > 0 {
            assignment_spells - 1
        } else {
            0
        };
        let mut new_delta_money = SideArray::new(0, 0);
        if spells_bought > 0 {
            new_delta_money[side] = -(spells_bought as i32 * spell_cost);
        }

        let mut new_tech_state = self.tech_state.clone();
        if new_tech_state
            .assign_techs(assignment.clone(), side, techline)
            .is_err()
        {
            return (
                TechAssignment::new(0, vec![]),
                Self::new(self.side, self.tech_state.clone(), SideArray::new(0, 0)),
            );
        }

        let new_node_state = Self::new(!self.side, new_tech_state, new_delta_money);

        (assignment, new_node_state)
    }
}

pub type GeneralNode<'a> = MCTSNode<'a, GeneralNodeState, TechAssignment>;
pub type GeneralNodeRef<'a> = &'a RefCell<GeneralNode<'a>>;
