use crate::heuristics::GeneralHeuristic;
use bumpalo::{collections::Vec as BumpVec, Bump};
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
use std::marker::PhantomData;

use super::mcts::{ChildGen, MCTSNode};

#[derive(Clone)]
pub struct GeneralPreTurn {
    pub weights: StdVec<f32>,
}

#[derive_where(PartialEq, Eq)]
pub struct GeneralNodeState<'a, C, H: GeneralHeuristic<'a, C>> {
    #[derive_where(skip)]
    pub heuristic_state: H::GeneralEnc,
    pub side: Side,
    pub tech_state: TechState,
    pub delta_money: SideArray<i32>,
}

impl<'a, C, H: GeneralHeuristic<'a, C>> GeneralNodeState<'a, C, H> {
    pub fn new(
        side: Side,
        tech_state: TechState,
        delta_money: SideArray<i32>,
        heuristic: &H,
    ) -> Self {
        Self {
            heuristic_state: heuristic.compute_enc(side, &tech_state),
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
    nodes: StdVec<DecisionNode>,
    _phantom: &'a (),
}

impl<'a> GeneralChildGen<'a> {
    fn expand_node<C, H: GeneralHeuristic<'a, C>>(
        &mut self,
        node_idx: usize,
        state: &GeneralNodeState<'a, C, H>,
        config: &GameConfig,
        max_techs: usize,
    ) {
        let techline = &config.techline;
        let num_techs = techline.len();

        // Reverse numerical ordering
        let tech_idx = num_techs - 1 - self.nodes[node_idx].depth;

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

        let tech = techline.techs[tech_idx];
        let already_ours = state.tech_state.acquired_techs[state.side].contains(&tech);
        let already_theirs = state.tech_state.acquired_techs[!state.side].contains(&tech);
        let invalid = already_ours || already_theirs;

        let mut take_saturated = new_num_techs > max_techs || invalid;

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
                .take_child.map(|i| self.nodes[i].saturated)
                .unwrap();

            if !left_sat {
                break;
            }

            let right_sat = self.nodes[parent_idx]
                .skip_child.map(|i| self.nodes[i].saturated)
                .unwrap();

            if !right_sat {
                break;
            }

            self.nodes[parent_idx].saturated = true;
            idx = parent_idx;
        }
    }

    fn propose_assignment<C: 'a, H: GeneralHeuristic<'a, C>>(
        &mut self,
        state: &GeneralNodeState<'a, C, H>,
        rng: &mut impl Rng,
        args: <Self as ChildGen<GeneralNodeState<'a, C, H>, TechAssignment>>::Args,
    ) -> Option<TechAssignment> {
        let GeneralNodeArgs {
            money,
            config,
            heuristic,
            pre_turn,
            _phantom,
        } = args;
        let spell_cost = config.spell_cost();
        let max_techs = 1 + (money / spell_cost) as usize;

        if self.nodes[0].saturated {
            // No more to explore
            return None;
        }

        // Traverse decision tree
        let mut cur_idx = 0;
        let num_techs = config.techline.len();

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

            cur_idx = if take_sat {
                skip_idx
            } else if skip_sat {
                take_idx
            } else {
                // choose using weight as probability to select
                let depth = self.nodes[cur_idx].depth;
                let tech_idx = num_techs - 1 - depth;
                let p_select = pre_turn.weights[tech_idx];

                if rng.random::<f32>() < p_select {
                    take_idx
                } else {
                    skip_idx
                }
            };
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

#[derive_where(Clone)]
pub struct GeneralNodeArgs<'a, C, H: GeneralHeuristic<'a, C>> {
    pub money: i32,
    pub config: &'a GameConfig,
    pub heuristic: &'a H,
    pub pre_turn: GeneralPreTurn,
    pub _phantom: PhantomData<C>,
}

impl<'a, C: 'a, H: GeneralHeuristic<'a, C>> ChildGen<GeneralNodeState<'a, C, H>, TechAssignment>
    for GeneralChildGen<'a>
{
    type Args = GeneralNodeArgs<'a, C, H>;

    fn new(_state: &GeneralNodeState<'a, C, H>, _rng: &mut impl Rng, _args: Self::Args) -> Self {
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
            nodes,
            _phantom: &(),
        }
    }

    fn propose_turn(
        &mut self,
        state: &GeneralNodeState<'a, C, H>,
        rng: &mut impl Rng,
        args: Self::Args,
    ) -> Option<(TechAssignment, GeneralNodeState<'a, C, H>)> {
        let assignment = self.propose_assignment(state, rng, args.clone())?;

        let mut new_tech_state = state.tech_state.clone();
        new_tech_state
            .assign_techs(assignment.clone(), state.side, &args.config.techline)
            .unwrap();

        let mut new_delta_money = SideArray::new(0, 0);
        new_delta_money[state.side] =
            -(assignment.num_spells() - 1).max(0) * args.config.spell_cost();

        let new_heuristic_state = args
            .heuristic
            .update_enc(&state.heuristic_state, &assignment);

        let new_node_state = GeneralNodeState {
            side: !state.side,
            tech_state: new_tech_state,
            delta_money: new_delta_money,
            heuristic_state: new_heuristic_state,
        };

        Some((assignment, new_node_state))
    }
}

pub type GeneralNode<'a, C, H> =
    MCTSNode<'a, GeneralNodeState<'a, C, H>, TechAssignment, GeneralChildGen<'a>>;
pub type GeneralNodeRef<'a, C, H> = &'a RefCell<GeneralNode<'a, C, H>>;
