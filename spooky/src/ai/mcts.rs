//! Strategic decision making across multiple boards

use crate::heuristics::Heuristic;
use std::{cell::RefCell, collections::HashMap};

use crate::core::{GameConfig, GameState, GameTurn, Side, SideArray};

use super::{
    blotto, // Assuming we'll have a blotto function/module here
    eval::Eval,
};

use rand::prelude::*;

use bumpalo::{collections::Vec, Bump};

#[derive(Copy, Clone, Debug, Default)]
pub struct NodeStats {
    pub visits: u32,
    pub eval: Option<Eval>,
}

impl NodeStats {
    pub fn new() -> Self {
        Self {
            visits: 0,
            eval: None,
        }
    }

    pub fn uct(&self, ln_n: f32, rng: &mut impl Rng, side: Side) -> f32 {
        const C: f32 = 0.1;

        let w = self.eval.unwrap().score(side);
        let n0 = self.visits as f32;

        w + C * (ln_n / n0).sqrt()
    }

    pub fn update(&mut self, eval: &Eval) {
        let n = self.visits as f32;

        self.visits += 1;
        match &mut self.eval {
            Some(e) => e.update_weighted(n, eval),
            None => self.eval = Some(eval.clone()),
        }
    }
}

pub trait ChildGen<State, Turn> {
    type Args: Clone;

    fn new(state: &State, rng: &mut impl Rng, args: Self::Args) -> Self;

    fn propose_turn(
        &mut self,
        state: &State,
        rng: &mut impl Rng,
        args: Self::Args,
    ) -> Option<(Turn, State)>;
}

#[derive(Debug)]
pub struct MCTSEdge<'a, Node, Turn> {
    pub turn: Turn,
    pub child: &'a RefCell<Node>,
}

#[derive(Debug)]
pub struct MCTSNode<'a, State, Turn, Gen: ChildGen<State, Turn>> {
    pub stats: NodeStats,
    pub phantom_stats: NodeStats,
    pub update_phantom: bool,
    pub edges: Vec<'a, MCTSEdge<'a, Self, Turn>>,
    pub state: State,
    pub side: Side,
    pub gen: Option<Gen>,
    pub exhausted: bool,
}

impl<'a, State: Eq, Turn, Gen: ChildGen<State, Turn>> MCTSNode<'a, State, Turn, Gen> {
    pub fn new(state: State, side: Side, arena: &'a Bump) -> Self {
        Self {
            stats: NodeStats::new(),
            phantom_stats: NodeStats::new(),
            update_phantom: true,
            edges: Vec::new_in(arena),
            state,
            side,
            gen: None,
            exhausted: false,
        }
    }

    fn children(&self) -> impl Iterator<Item = &RefCell<Self>> {
        self.edges.iter().map(|edge| edge.child)
    }

    /// returns (whether a new child was made, index of selected child)
    pub fn poll<H: Heuristic<'a>>(
        &mut self,
        arena: &'a Bump,
        heuristic: &'a H,
        rng: &mut impl Rng,
        args: Gen::Args,
    ) -> (bool, usize)
    where
        Self: 'a,
    {
        // initialize generator on first poll and return first child
        if self.gen.is_none() {
            self.gen = Some(Gen::new(&self.state, rng, args.clone()));
            // child must be made
            return (true, self.make_child(arena, heuristic, rng, args).unwrap());
        }

        let mut best_child_index = 0;
        let mut best_uct = f32::NEG_INFINITY;
        let ln_n = (self.stats.visits as f32).ln();

        for (i, child) in self.children().enumerate() {
            let child = child.borrow();
            let stats = child.stats;

            let uct = stats.uct(ln_n, rng, self.side);
            if uct > best_uct {
                best_child_index = i;
                best_uct = uct;
            }
        }

        if !self.exhausted {
            let phantom_uct = self.phantom_stats.uct(ln_n, rng, self.side);

            if best_uct <= phantom_uct {
                self.update_phantom = true;
                if let Some(idx) = self.make_child(arena, heuristic, rng, args) {
                    return (true, idx);
                }
            }
            self.exhausted = true;
        }

        self.update_phantom = false;
        (false, best_child_index)
    }

    pub fn update(&mut self, eval: &Eval) {
        self.stats.update(eval);
        if self.update_phantom {
            self.phantom_stats.update(eval);
        }
    }

    // returns None if no child was made
    // returns Some(index) if a child was made at that index
    pub fn make_child<H: Heuristic<'a>>(
        &mut self,
        arena: &'a Bump,
        heuristic: &'a H,
        rng: &mut impl Rng,
        args: Gen::Args,
    ) -> Option<usize>
    where
        Self: 'a,
    {
        let (turn, new_node_state) = self
            .gen
            .as_mut()
            .expect("Child generator must be initialized")
            .propose_turn(&self.state, rng, args)?;

        // Check if this state already exists as a child to avoid duplicates.
        let child_mcts_node = self
            .edges
            .iter()
            .find_map(|edge| {
                let child = edge.child.borrow();
                if child.state == new_node_state {
                    Some(edge.child)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                let new_mcts_node = MCTSNode::new(new_node_state, self.side, arena);
                let new_mcts_node_ref = arena.alloc(RefCell::new(new_mcts_node));
                new_mcts_node_ref
            });

        let edge = MCTSEdge {
            turn,
            child: child_mcts_node,
        };

        self.edges.push(edge);

        Some(self.edges.len() - 1)
    }

    pub fn best_turn(&self) -> Turn
    where
        Turn: Clone,
    {
        self.edges
            .iter()
            .max_by_key(|edge| edge.child.borrow().stats.visits)
            .expect("No children to select best turn from")
            .turn
            .clone()
    }

    pub fn print_tree(&self, indent: usize) -> String {
        let mut tree = String::new();
        tree.push_str(&format!("{:indent$}{:?}\n", "", self.stats));
        for edge in &self.edges {
            tree.push_str(&edge.child.borrow().print_tree(indent + 2));
        }
        tree
    }
}
