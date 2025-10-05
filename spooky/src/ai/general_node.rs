use crate::heuristics::GeneralHeuristic;
use bumpalo::{collections::Vec, Bump};
use derive_where::derive_where;
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

#[derive_where(Clone)]
pub struct GeneralNodeState<'a, H: GeneralHeuristic<'a>> {
    pub heuristic_state: H::GeneralEnc,
    pub side: Side,
    pub tech_state: TechState,
    pub delta_money: SideArray<i32>,
}

impl<'a, H: GeneralHeuristic<'a>> PartialEq for GeneralNodeState<'a, H> {
    fn eq(&self, other: &Self) -> bool {
        self.side == other.side
            && self.tech_state == other.tech_state
            && self.delta_money == other.delta_money
    }
}

impl<'a, H: GeneralHeuristic<'a>> Eq for GeneralNodeState<'a, H> {}

impl<'a, H: GeneralHeuristic<'a>> GeneralNodeState<'a, H> {
    pub fn new(
        side: Side,
        tech_state: TechState,
        delta_money: SideArray<i32>,
        heuristic: &H,
    ) -> Self {
        Self {
            heuristic_state: heuristic.compute_enc(&tech_state),
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
pub struct GeneralChildGen<'a> {
    // indices into techline, sorted by weight desc
    order: StdVec<usize>,
    // normalized weights in [0,1]
    weights: StdVec<f32>,
    nodes: StdVec<DecisionNode>,
    _phantom: &'a (),
}

impl<'a> GeneralChildGen<'a> {
    fn expand_node<H: GeneralHeuristic<'a>>(
        &mut self,
        node_idx: usize,
        state: &GeneralNodeState<'a, H>,
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

    fn propose_assignment<H: GeneralHeuristic<'a>>(
        &mut self,
        state: &GeneralNodeState<'a, H>,
        rng: &mut impl Rng,
        args: <Self as ChildGen<GeneralNodeState<'a, H>, TechAssignment>>::Args,
    ) -> Option<TechAssignment> {
        let (money, config, _heuristic) = args;
        let spell_cost = config.spell_cost();
        let max_techs = 1 + (money / spell_cost) as usize;

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

impl<'a, H: GeneralHeuristic<'a>> ChildGen<GeneralNodeState<'a, H>, TechAssignment>
    for GeneralChildGen<'a>
{
    type Args = (i32, &'a GameConfig, &'a H);

    fn new(state: &GeneralNodeState<'a, H>, rng: &mut impl Rng, args: Self::Args) -> Self {
        let (money, config, heuristic) = args;
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
            _phantom: &(),
        }
    }

    fn propose_turn(
        &mut self,
        state: &GeneralNodeState<'a, H>,
        rng: &mut impl Rng,
        args: Self::Args,
    ) -> Option<(TechAssignment, GeneralNodeState<'a, H>)> {
        let (_money, _config, heuristic) = args;
        let assignment = self.propose_assignment(state, rng, args)?;

        let mut new_tech_state = state.tech_state.clone();
        new_tech_state
            .assign_techs(assignment.clone(), state.side, &args.1.techline)
            .unwrap();

        let mut new_delta_money = SideArray::new(0, 0);
        new_delta_money[state.side] = -(assignment.num_spells() - 1).max(0) * args.1.spell_cost();

        let new_heuristic_state = heuristic.update_enc(&state.heuristic_state, &assignment);

        let new_node_state = GeneralNodeState {
            side: !state.side,
            tech_state: new_tech_state,
            delta_money: new_delta_money,
            heuristic_state: new_heuristic_state,
        };

        Some((assignment, new_node_state))
    }
}

pub type GeneralNode<'a, H> =
    MCTSNode<'a, GeneralNodeState<'a, H>, TechAssignment, GeneralChildGen<'a>>;

pub type GeneralNodeRef<'a, H> = &'a RefCell<GeneralNode<'a, H>>;
