use bumpalo::{collections::Vec, Bump};
use std::cell::RefCell;
use std::vec::Vec as StdVec;

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
    mask: u32, // bitmask over tech indices (0..NUM_TECHS-1)
    selected: usize,
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
    fn spells_needed(unlock: usize, mask: u32) -> usize {
        if mask == 0 {
            return 0;
        }
        let max_idx = (0..NUM_TECHS)
            .rev()
            .find(|&i| (mask & (1u32 << i)) != 0)
            .unwrap();
        let advance = (max_idx as i32 - unlock as i32 + 1).max(0) as usize;
        let acquires = mask.count_ones() as usize;
        advance + acquires
    }

    fn tech_conflict(
        tech_state: &TechState,
        side: Side,
        techline: &crate::core::tech::Techline,
        idx: usize,
        mask: u32,
    ) -> bool {
        // Cannot acquire if already acquired by us or enemy (unless Copycat is already acquired or included)
        let tech = techline[idx];
        let ours = tech_state.acquired_techs[side].contains(&tech);
        if ours {
            return true;
        }
        let theirs = tech_state.acquired_techs[!side].contains(&tech);
        if theirs {
            // allowed only if we already have or are taking Copycat
            let copycat_idx = techline.index_of(Tech::Copycat);
            let have_copycat = tech_state.acquired_techs[side].contains(&Tech::Copycat)
                || ((mask & (1u32 << copycat_idx)) != 0);
            return !have_copycat;
        }
        false
    }

    fn expand_node(
        &mut self,
        node_idx: usize,
        state: &GeneralNodeState,
        config: &GameConfig,
        max_techs: usize,
    ) {
        let techline = &config.techline;
        let order_len = self.order.len();

        // Check if already visited or at end
        if self.nodes[node_idx].visited || self.nodes[node_idx].depth >= order_len {
            self.nodes[node_idx].visited = true;
            return;
        }

        // Extract values we need before creating children
        let cur_depth = self.nodes[node_idx].depth;
        let node_mask = self.nodes[node_idx].mask;
        let node_selected = self.nodes[node_idx].selected;
        self.nodes[node_idx].visited = true;
        let tech_idx = self.order[cur_depth];

        // create skip child (do not take this tech)
        let skip_child = DecisionNode {
            depth: cur_depth + 1,
            mask: node_mask,
            selected: node_selected,
            visited: false,
            saturated: cur_depth + 1 >= order_len, // at end => leaf
            parent: Some(node_idx),
            take_child: None,
            skip_child: None,
        };
        let skip_idx = self.nodes.len();
        self.nodes.push(skip_child);
        self.nodes[node_idx].skip_child = Some(skip_idx);

        // create take child if possible under constraints
        let mut take_saturated = false;
        let new_mask = node_mask | (1u32 << tech_idx);
        // cannot select if conflicts
        if Self::tech_conflict(&state.tech_state, state.side, techline, tech_idx, node_mask) {
            take_saturated = true;
        }
        let spells = Self::spells_needed(state.tech_state.unlock_index[state.side], new_mask);
        if spells > max_techs {
            take_saturated = true;
        }

        let take_child = DecisionNode {
            depth: cur_depth + 1,
            mask: new_mask,
            selected: node_selected + 1,
            visited: false,
            saturated: take_saturated || cur_depth + 1 >= order_len,
            parent: Some(node_idx),
            take_child: None,
            skip_child: None,
        };
        let take_idx = self.nodes.len();
        self.nodes.push(take_child);
        self.nodes[node_idx].take_child = Some(take_idx);
    }

    fn propagate_saturation(&mut self, mut idx: usize) {
        while let Some(parent_idx) = self.nodes[idx].parent {
            let left_sat = self.nodes[parent_idx]
                .take_child
                .and_then(|i| Some(self.nodes[i].saturated))
                .unwrap_or(true);
            let right_sat = self.nodes[parent_idx]
                .skip_child
                .and_then(|i| Some(self.nodes[i].saturated))
                .unwrap_or(true);
            if left_sat && right_sat {
                if !self.nodes[parent_idx].saturated {
                    self.nodes[parent_idx].saturated = true;
                }
                idx = parent_idx;
                continue;
            }
            break;
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
        let max_techs = 1 + (*money as usize / spell_cost as usize);

        if self.nodes[0].saturated {
            // No more to explore
            return None;
        }

        // Traverse decision tree
        let mut cur_idx = 0usize;
        loop {
            let order_len = self.order.len();
            if self.nodes[cur_idx].depth >= order_len {
                break;
            }

            if !self.nodes[cur_idx].visited {
                self.expand_node(cur_idx, state, config, max_techs);
                // if take child impossible -> mark it saturated
                if let Some(take_idx) = self.nodes[cur_idx].take_child {
                    // already marked saturated if impossible
                    let _ = take_idx;
                }
            }

            // If any child is saturated, take the other
            let take_idx = self.nodes[cur_idx].take_child.unwrap();
            let skip_idx = self.nodes[cur_idx].skip_child.unwrap();
            let take_sat = self.nodes[take_idx].saturated;
            let skip_sat = self.nodes[skip_idx].saturated;

            let next_idx = if take_sat && !skip_sat {
                skip_idx
            } else if skip_sat && !take_sat {
                take_idx
            } else if take_sat && skip_sat {
                // both saturated -> mark current saturated and stop
                self.nodes[cur_idx].saturated = true;
                break;
            } else {
                // choose using weight as probability to select
                let depth = self.nodes[cur_idx].depth;
                let tech_idx = self.order[depth];
                let p_select = self
                    .weights
                    .get(tech_idx)
                    .copied()
                    .unwrap_or(0.5)
                    .clamp(0.0, 1.0);
                if rng.gen::<f32>() < p_select {
                    take_idx
                } else {
                    skip_idx
                }
            };

            cur_idx = next_idx;

            // Termination conditions
            let at_last = self.nodes[cur_idx].depth >= self.order.len();
            let unlock = state.tech_state.unlock_index[state.side];
            let spells = Self::spells_needed(unlock, self.nodes[cur_idx].mask);
            if at_last || spells >= max_techs {
                break;
            }
        }

        // Mark leaf saturated and propagate
        self.nodes[cur_idx].saturated = true;
        self.propagate_saturation(cur_idx);

        // Convert path mask to set of techs
        let mask = self.nodes[cur_idx].mask;
        let mut acquire: StdVec<usize> = StdVec::new();
        for i in 0..NUM_TECHS {
            if (mask & (1u32 << i)) != 0 {
                acquire.push(i);
            }
        }

        // Never march unless not teching at all: only advance to minimally unlock if acquiring.
        let unlock = state.tech_state.unlock_index[state.side];
        let advance_by = if acquire.is_empty() {
            0
        } else {
            let max_idx = *acquire.iter().max().unwrap();
            (max_idx as i32 - unlock as i32 + 1).max(0) as usize
        };
        let assignment = TechAssignment::new(advance_by, acquire);

        // Money delta: pay for extra spells beyond 1 free
        let total_spells = assignment.num_spells();
        let spells_bought = if total_spells > 0 {
            total_spells - 1
        } else {
            0
        };
        let mut new_delta_money = SideArray::new(0, 0);
        if spells_bought > 0 {
            new_delta_money[state.side] = -(spells_bought as i32 * spell_cost);
        }

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
            .map(|(i, t)| {
                let already_ours = state.tech_state.acquired_techs[state.side].contains(t);
                if already_ours {
                    return (i, 0.0);
                }
                let already_theirs = state.tech_state.acquired_techs[!state.side].contains(t);
                if already_theirs {
                    return (i, 0.0);
                }
                (i, rng.gen_range(0.0..1.0))
            })
            .collect();

        // Sort by weight desc
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let maxw = scored.first().map(|x| x.1).unwrap_or(1.0).max(1e-6);

        let order = scored.iter().map(|(i, _)| *i).collect();
        let weights = scored
            .iter()
            .map(|(_, w)| (w / maxw).clamp(0.0, 1.0))
            .collect();

        // root node
        let nodes = vec![DecisionNode {
            depth: 0,
            mask: 0,
            selected: 0,
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
