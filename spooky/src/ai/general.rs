use bumpalo::{collections::Vec, Bump};
use std::vec::Vec as StdVec;
use std::{cell::RefCell, collections::HashSet};

use crate::core::{
    convert::ToIndex,
    side::SideArray,
    tech::{TechAssignment, TechState, NUM_TECHS},
    FromIndex, GameConfig, Side, Tech, Unit,
};

use rand::prelude::*;

use super::mcts::{ChildGen, MCTSNode};

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

// --------------------------
// Child generator for Generaling
// --------------------------

#[derive(Clone, Debug)]
struct DecisionNode {
    depth: usize,
    tech_indices: HashSet<usize>,
    advance_by: usize,
    visited: bool,
    saturated: bool,
    parent: Option<usize>,
    take_child: Option<usize>,
    skip_child: Option<usize>,
}

#[derive(Debug)]
pub struct GeneralChildGen {
    // indices into techline, sorted by weight desc
    order: StdVec<usize>,
    // normalized weights in [0,1]
    weights: StdVec<f32>,
    nodes: StdVec<DecisionNode>,
}

impl GeneralChildGen {
    fn expand_node(
        &mut self,
        node_idx: usize,
        state: &GeneralNodeState,
        config: &GameConfig,
        max_techs: usize,
    ) {
        let techline = &config.techline;
        let num_techs = self.order.len();

        let tech_idx = self.order[self.nodes[node_idx].depth];

        let new_depth = self.nodes[node_idx].depth + 1;
        let mut new_tech_indices = self.nodes[node_idx].tech_indices.clone();
        let advance_by = self.nodes[node_idx].advance_by;

        // create skip child (do not take this tech)
        let skip_child = DecisionNode {
            depth: new_depth,
            tech_indices: new_tech_indices.clone(),
            advance_by,
            visited: false,
            saturated: false,
            parent: Some(node_idx),
            take_child: None,
            skip_child: None,
        };

        // create take child
        new_tech_indices.insert(tech_idx);
        let advance_by_for_tech =
            tech_idx + 1 - state.tech_state.unlock_index[state.side].min(tech_idx + 1);
        let new_advance_by = advance_by.max(advance_by_for_tech);
        let new_num_techs = new_tech_indices.len() + new_advance_by;
        let mut take_saturated = new_num_techs > max_techs;

        let take_child = DecisionNode {
            depth: new_depth,
            tech_indices: new_tech_indices,
            advance_by: new_advance_by,
            visited: false,
            saturated: take_saturated,
            parent: Some(node_idx),
            take_child: None,
            skip_child: None,
        };

        self.nodes[node_idx].visited = true;

        let skip_idx = self.nodes.len();
        self.nodes.push(skip_child);
        self.nodes[node_idx].skip_child = Some(skip_idx);

        let take_idx = self.nodes.len();
        self.nodes.push(take_child);
        self.nodes[node_idx].take_child = Some(take_idx);
    }

    fn propagate_saturation(&mut self, mut idx: usize) {
        self.nodes[idx].saturated = true;

        while let Some(parent_idx) = self.nodes[idx].parent {
            let left_sat = self.nodes[parent_idx]
                .take_child
                .and_then(|i| Some(self.nodes[i].saturated))
                .unwrap();

            if !left_sat {
                break;
            }

            let right_sat = self.nodes[parent_idx]
                .skip_child
                .and_then(|i| Some(self.nodes[i].saturated))
                .unwrap();

            if !right_sat {
                break;
            }

            self.nodes[parent_idx].saturated = true;
            idx = parent_idx;
        }
    }

    fn propose_assignment(
        &mut self,
        state: &GeneralNodeState,
        rng: &mut impl Rng,
        args: &<Self as ChildGen<GeneralNodeState, TechAssignment>>::Args,
    ) -> Option<TechAssignment> {
        let (money, config) = args;
        let spell_cost = config.spell_cost();
        let max_techs = 1 + (*money / spell_cost) as usize;

        if self.nodes[0].saturated {
            // No more to explore
            return None;
        }

        // Traverse decision tree
        let mut cur_idx = 0;
        let num_techs = self.order.len();

        loop {
            if self.nodes[cur_idx].depth >= num_techs {
                break;
            }

            if self.nodes[cur_idx].tech_indices.len() + self.nodes[cur_idx].advance_by >= max_techs
            {
                break;
            }

            if !self.nodes[cur_idx].visited {
                self.expand_node(cur_idx, state, config, max_techs);
            }

            // If any child is saturated, take the other
            let take_idx = self.nodes[cur_idx].take_child.unwrap();
            let skip_idx = self.nodes[cur_idx].skip_child.unwrap();

            let take_sat = self.nodes[take_idx].saturated;
            let skip_sat = self.nodes[skip_idx].saturated;

            let next_idx = if take_sat {
                skip_idx
            } else if skip_sat {
                take_idx
            } else {
                // choose using weight as probability to select
                let depth = self.nodes[cur_idx].depth;
                let p_select = self.weights[depth];

                if rng.gen::<f32>() < p_select {
                    take_idx
                } else {
                    skip_idx
                }
            };

            cur_idx = next_idx;
        }

        // Mark leaf saturated and propagate
        self.propagate_saturation(cur_idx);

        // Convert path mask to set of techs
        let acquire: StdVec<usize> = self.nodes[cur_idx].tech_indices.iter().cloned().collect();
        let mut advance_by = self.nodes[cur_idx].advance_by;
        let num_spells = acquire.len() + advance_by;
        if num_spells == 0 && state.tech_state.unlock_index[state.side] < config.techline.len() {
            advance_by = 1;
        }
        let assignment = TechAssignment::new(advance_by, acquire);

        Some(assignment)
    }
}

impl ChildGen<GeneralNodeState, TechAssignment> for GeneralChildGen {
    type Args = (i32, GameConfig);

    fn new(state: &GeneralNodeState, rng: &mut impl Rng, args: &Self::Args) -> Self {
        let (money, config) = args;
        let techline = &config.techline;
        let unlock = state.tech_state.unlock_index[state.side];
        let mut scored: StdVec<(usize, f32)> = techline
            .techs
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let already_ours = state.tech_state.acquired_techs[state.side].contains(t);
                if already_ours {
                    return None;
                }
                let already_theirs = state.tech_state.acquired_techs[!state.side].contains(t);
                if already_theirs {
                    return None;
                }
                Some((i, rng.gen_range(0.0..1.0)))
            })
            .collect();

        // Sort by weight desc
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let (order, weights) = scored.into_iter().unzip();

        // root node
        let nodes = vec![DecisionNode {
            depth: 0,
            tech_indices: HashSet::new(),
            advance_by: 0,
            visited: false,
            saturated: false,
            parent: None,
            take_child: None,
            skip_child: None,
        }];

        GeneralChildGen {
            order,
            weights,
            nodes,
        }
    }

    fn propose_turn(
        &mut self,
        state: &GeneralNodeState,
        rng: &mut impl Rng,
        args: &Self::Args,
    ) -> Option<(TechAssignment, GeneralNodeState)> {
        let assignment = self.propose_assignment(state, rng, args)?;

        let mut new_tech_state = state.tech_state.clone();
        new_tech_state
            .assign_techs(assignment.clone(), state.side, &args.1.techline)
            .unwrap();

        let mut new_delta_money = SideArray::new(0, 0);
        new_delta_money[state.side] = -(assignment.num_spells() - 1).max(0) * args.1.spell_cost();

        let new_node_state = GeneralNodeState::new(!state.side, new_tech_state, new_delta_money);
        Some((assignment, new_node_state))
    }
}

pub type GeneralNode<'a> = MCTSNode<'a, GeneralNodeState, TechAssignment, GeneralChildGen>;
pub type GeneralNodeRef<'a> = &'a RefCell<GeneralNode<'a>>;
