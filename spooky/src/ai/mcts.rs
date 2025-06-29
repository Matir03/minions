//! Strategic decision making across multiple boards

use std::{
    cell::RefCell,
    collections::HashMap,
};

use crate::core::{GameConfig, GameState, Side, SideArray, GameTurn};

use super::{
    blotto, // Assuming we'll have a blotto function/module here
    eval::Eval,
    search::SearchArgs,
};

use rand::prelude::*;

use bumpalo::{Bump, collections::Vec};

#[derive(Copy, Clone, Debug, Default)]
pub struct NodeStats {
    pub visits: u32,
    /// stores eval from the perspective of the _previous_ node
    pub eval: Eval,
}

impl NodeStats {
    pub fn new() -> Self {
        Self {
            visits: 0,
            eval: Eval::new(0.0),
        }
    }

    pub fn uct(&self, ln_n: f32, rng: &mut impl Rng) -> f32 {
        const C: f32 = 1.5;
        const EPS: f32 = 0.1;

        let w = self.eval.winprob;
        let n0 = self.visits as f32;
        let r = rng.gen_range(-EPS..EPS);

        w + C * (ln_n / (n0 + 1.0)).sqrt() + r
    }

    pub fn update(&mut self, eval: &Eval) {
        let n = self.visits as f32;

        self.visits += 1;
        self.eval.winprob = (self.eval.winprob * n + eval.winprob) / (n + 1.0);
    }
}

pub trait NodeState<Turn> {
    type Args;

    fn propose_move(&self, rng: &mut impl Rng, args: &Self::Args) -> (Turn, Self);
}

#[derive(Debug)]
pub struct MCTSEdge<'a, Node, Turn> {
    pub turn: Turn,
    pub child: &'a RefCell<Node>,
}

#[derive(Debug)]
pub struct MCTSNode<'a, State: NodeState<Turn>, Turn> {
    pub stats: NodeStats,
    pub edges: Vec<'a, MCTSEdge<'a, Self, Turn>>,
    pub state: State,
}

impl<'a, State: NodeState<Turn> + PartialEq, Turn> MCTSNode<'a, State, Turn> {
    pub fn new(state: State, arena: &'a Bump) -> Self {
        Self {
            stats: NodeStats::new(),
            edges: Vec::new_in(arena),
            state,
        }
    }

    fn children(&self) -> impl Iterator<Item = &RefCell<Self>> {
        self.edges.iter()
            .map(|edge| 
                edge.child)
    }

    /// (whether a new child was made, index of selected child)
    pub fn poll(&mut self, search_args: &SearchArgs<'a>, rng: &mut impl Rng, args: State::Args) -> (bool, usize)
        where Self: 'a {

        // If no children exist, always create a new one
        if self.edges.is_empty() {
            let child_index = self.make_child(search_args, rng, args);
            return (true, child_index);
        }

        let mut best_child_index = 0;
        let mut best_uct = f32::NEG_INFINITY;
        let mut total_score: f32 = 0.0;
        let ln_n = (self.stats.visits as f32).ln();

        for (i, child) in self.children().enumerate() {
            let child = child.borrow();
            let stats = child.stats;

            let uct = stats.uct(ln_n, rng);
            if uct > best_uct {
                best_child_index = i;
                best_uct = uct;
            }

            let score = stats.eval.winprob();

            total_score += score;
        }

        let num_children = self.edges.len();
        let avg_score = total_score / num_children.min(1) as f32;

        let phantom_stats = NodeStats {
            visits: 1 + num_children as u32,
            eval: Eval::new(avg_score),
        };

        let phantom_uct = phantom_stats.uct(ln_n, rng);

        if best_uct <= phantom_uct {
            let child_index = self.make_child(search_args, rng, args);

            return (true, child_index);
        }

        (false, best_child_index)
    }

    // TODO: Review this method's logic, especially panic cases.
    fn get_child(&mut self, search_args: &SearchArgs<'a>, rng: &mut impl Rng, args: State::Args) -> (usize, &'a RefCell<Self>)
        where Self: 'a {

        // Ensure we have at least one child
        if self.edges.is_empty() {
            let index = self.make_child(search_args, rng, args);
            return (index, self.edges[index].child);
        }

        let (_is_new, index) = self.poll(search_args, rng, args);

        // Ensure the index is valid. This should be guaranteed by the logic in `poll`.
        if index >= self.edges.len() {
            unreachable!("get_child: poll() returned an invalid index ({}) but edges has length {}", index, self.edges.len());
        }

        (index, self.edges[index].child)
    }

    pub fn update(&mut self, eval: &Eval) {
        self.stats.update(eval);
    }

    pub fn make_child(&mut self, search_args: &SearchArgs<'a>, rng: &mut impl Rng, args: State::Args) -> usize
        where Self: 'a, State: PartialEq
    {
        let (turn, new_node_state) = self.state.propose_move(rng, &args); // Args might need to be Clone

        // Check if this turn/state already exists as a child to avoid duplicates.
        if let Some(pos) = self.edges.iter().position(|edge| edge.child.borrow().state == new_node_state) {
            return pos;
        }

        let new_mcts_node = MCTSNode::new(new_node_state, search_args.arena); 
        let new_mcts_node_ref = search_args.arena.alloc(RefCell::new(new_mcts_node));

        let edge = MCTSEdge {
            turn,
            child: new_mcts_node_ref,
        };

        self.edges.push(edge);

        self.edges.len() - 1
    }

    pub fn best_turn(&self) -> Turn where Turn: Clone {
        self.edges.iter()
            .max_by_key(|edge| edge.child.borrow().stats.visits)
            .expect("No children to select best turn from")
            .turn.clone()
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