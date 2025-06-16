//! Strategic decision making across multiple boards

use std::{
    cell::RefCell,
    collections::HashMap,
};

use crate::core::{GameConfig, GameState, Side, SideArray, Turn};

use super::{
    attack::{AttackNode, AttackNodeRef},
    blotto::blotto,
    eval::Eval,
    general::{GeneralNode, GeneralNodeRef},
    search::SearchArgs,
};

use rand::prelude::*;

use bumpalo::{Bump, collections::Vec};
use std::vec::Vec as StdVec;

#[derive(Debug, Clone, Copy)]
pub enum Stage {
    General,
    Attack,
    Blotto,
    Spawn,
}

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

        let w = self.eval.winprob();
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


pub trait MCTSNode<'a> {
    type Child: MCTSNode<'a>;
    type Etc;

    fn stats(&self) -> &NodeStats;
    fn children(&self) -> &StdVec<&'a RefCell<Self::Child>>;

    /// (whether a new child was made, index of selected child)
    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, etc: Self::Etc) -> (bool, usize);

    /// (whether a new child was made, index of selected child)
    fn poll(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, etc: Self::Etc) -> (bool, usize)
        where Self: 'a {

        // If no children exist, always create a new one
        if self.children().is_empty() {
            let (is_new, child_index) = self.make_child(args, rng, etc);
            return (is_new, child_index);
        }

        let mut best_child_index = 0;
        let mut best_uct = f32::NEG_INFINITY;
        let mut total_score: f32 = 0.0;
        let ln_n = (self.stats().visits as f32).ln();

        for (i, child) in self.children().iter().enumerate() {
            let child = child.borrow();
            let stats = child.stats();

            let uct = stats.uct(ln_n, rng);
            if uct > best_uct {
                best_child_index = i;
                best_uct = uct;
            }

            let score = stats.eval.winprob();

            total_score += score;
        }

        let avg_score = total_score / self.children().len().min(1) as f32;

        let phantom_stats = NodeStats {
            visits: 1 + self.children().len() as u32,
            eval: Eval::new(avg_score),
        };

        let phantom_uct = phantom_stats.uct(ln_n, rng);

        if best_uct <= phantom_uct {
            let (is_new, child_index) = self.make_child(args, rng, etc);

            if is_new {
                return (true, child_index);
            }
        }

        (false, best_child_index)
    }

    fn get_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, etc: Self::Etc) -> (usize, &'a RefCell<Self::Child>)
        where Self: 'a {

        // Ensure we have at least one child
        if self.children().is_empty() {
            let (is_new, index) = self.make_child(args, rng, etc);
            if !is_new {
                // If make_child failed, create a dummy child or panic
                panic!("Failed to create child node");
            }
            return (index, self.children()[index]);
        }

        let (is_new, index) = self.poll(args, rng, etc);

        // Ensure the index is valid
        if index >= self.children().len() {
            panic!("Invalid child index: {} >= {}", index, self.children().len());
        }

        (index, self.children()[index])
    }

    fn update(&mut self, eval: &Eval);

    // fn get_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng) -> (usize, Self::Child)
    // where Self: 'a {
    //     let (_, child_index) = self.poll(args, rng);

    //     (child_index, self.children()[child_index])
    // }
}