use bumpalo::{collections::Vec, Bump};
use std::cell::RefCell;
use std::vec::Vec as StdVec;

use crate::core::{
    convert::ToIndex,
    side::SideArray,
    tech::{TechAssignment, TechState, NUM_TECHS},
    FromIndex, GameConfig, Side, Tech, Unit,
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
    // (money, config)
    type Args = (i32, GameConfig);

    fn propose_move(&self, _rng: &mut impl Rng, args: &Self::Args) -> (TechAssignment, Self) {
        let (money, config) = args;
        let spell_cost = config.spell_cost();
        let num_buyable_spells = money / spell_cost;
        let techline = &config.techline;
        let side = self.side;

        let our_techs = &self.tech_state.acquired_techs[side];
        let enemy_techs_set = &self.tech_state.acquired_techs[!side];

        let mut enemy_techs: StdVec<_> = enemy_techs_set.iter().copied().collect();
        enemy_techs.sort();

        let mut highest_uncountered_enemy_tech = None;
        for &enemy_tech in enemy_techs.iter().rev() {
            let counters = enemy_tech.counters();
            let is_countered = counters.iter().any(|&c| our_techs.contains(&c));
            if !is_countered {
                highest_uncountered_enemy_tech = Some(enemy_tech);
                break;
            }
        }

        let mut target_tech = None;

        if let Some(enemy_tech) = highest_uncountered_enemy_tech {
            // Prioritize countering the highest uncountered enemy unit.
            let counters = enemy_tech.counters();
            println!("counters: {:?}", counters);
            target_tech = counters
                .into_iter()
                .filter(|&t| {
                    self.tech_state
                        .acquirable(t, techline, side, num_buyable_spells)
                })
                .max();
            println!("target_tech: {:?}", target_tech);
        } else {
            // If all enemy units are countered, press the advantage.
            if let Some(&our_highest_tech) = our_techs.iter().max() {
                let our_highest_tech_idx = our_highest_tech.to_index().unwrap();
                let n_plus_3 = our_highest_tech_idx + 3;
                let n_plus_5 = our_highest_tech_idx + 5;
                target_tech = [n_plus_3, n_plus_5]
                    .into_iter()
                    .filter_map(|i| Unit::from_index(i).ok())
                    .map(Tech::UnitTech)
                    .filter(|t| {
                        self.tech_state
                            .acquirable(*t, techline, side, num_buyable_spells)
                    })
                    .max();
            } else {
                // no techs: tech to the highest gettable tech
                target_tech = techline
                    .techs
                    .iter()
                    .filter(|t| {
                        self.tech_state
                            .acquirable(**t, techline, side, num_buyable_spells)
                    })
                    .copied()
                    .max();
            }
        }

        let assignment = if let Some(t) = target_tech {
            let idx = techline.index_of(t);
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
        new_tech_state
            .assign_techs(assignment.clone(), side, techline)
            .unwrap();

        let new_node_state = Self::new(!self.side, new_tech_state, new_delta_money);

        (assignment, new_node_state)
    }
}

pub type GeneralNode<'a> = MCTSNode<'a, GeneralNodeState, TechAssignment>;
pub type GeneralNodeRef<'a> = &'a RefCell<GeneralNode<'a>>;
