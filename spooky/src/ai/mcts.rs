//! Strategic decision making across multiple boards

use std::cell::RefCell;

use crate::core::{GameConfig, GameState, Side, SideArray, Turn};

use super::{
    search::SearchArgs,
    attack::{AttackNode, AttackNodeRef},
    general::{GeneralNode, GeneralNodeRef},
    eval::Eval,
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
    pub eval: Eval,
}

impl NodeStats {
    pub fn new() -> Self {
        Self {
            visits: 0,
            eval: Eval::new(0.0),
        }
    }
}

pub struct GameNode<'a> {
    pub stats: NodeStats,
    pub state: GameState,
    pub general_node: GeneralNodeRef<'a>,
    pub board_nodes: Vec<'a, AttackNodeRef<'a>>,
    pub children: Vec<'a, GameNode<'a>>,
}

impl <'a> GameNode<'a> {
    pub fn new(parent: &'a GameNode<'a>, general_node: GeneralNodeRef<'a>, board_nodes: Vec<'a, AttackNodeRef<'a>>, config: &GameConfig, arena: &'a Bump) -> Self {
        let mut board_points = parent.state.board_points.clone();
        let mut money = parent.state.money.clone();

        for b in &board_nodes {
            let b = b.borrow();
            board_points[Side::S0] += b.delta_points[Side::S0];
            board_points[Side::S1] += b.delta_points[Side::S1];

            money[Side::S0] += b.delta_money[Side::S0];
            money[Side::S1] += b.delta_money[Side::S1];
        }

        let g = general_node.borrow();

        money[Side::S0] += g.delta_money[Side::S0];
        money[Side::S1] += g.delta_money[Side::S1];

        let state = GameState {
            tech_state: g.tech_state.clone(),
            side_to_move: !parent.state.side_to_move,
            boards: board_nodes.iter().map(|b| b.borrow().board.clone()).collect(),
            board_points,
            money,
        };

        let eval = Eval::static_eval(config, &state);

        Self {
            stats: NodeStats {
                visits: 1,
                eval,
            },
            state,
            general_node,
            board_nodes,
            children: Vec::new_in(arena),
        }        
    }

    pub fn from_state(state: GameState, config: &GameConfig, arena: &'a Bump) -> Self {
        let state_clone = state.clone();
        let eval = Eval::static_eval(config, &state);

        let general_node = arena.alloc(RefCell::new(
            GeneralNode::new(state.tech_state, SideArray::new(0, 0), arena)));

        let board_nodes = Vec::from_iter_in(
            state.boards.into_iter()
                .map(|b| arena.alloc(RefCell::new(AttackNode::new(b, arena))) as AttackNodeRef), 
            arena);

        Self {
            stats: NodeStats {
                visits: 1,
                eval,
            },
            state: state_clone,
            general_node,
            board_nodes,
            children: Vec::new_in(arena),
        }
    }

    pub fn explore(&mut self, args: &mut SearchArgs<'a>) {
        let (is_new, explore_index) = self.poll(args);
        let next_node = &mut self.children[explore_index];

        if !is_new {
            next_node.explore(args);
        }

    }

    pub fn poll(&mut self, args: &mut SearchArgs<'a>) -> (bool, usize) {
        // let general_next = self.general_node.explore(arena);
        // let boards_next = self.board_nodes.iter_mut()
        //     .map(|n| n.explore(arena));
        let mut best_child_index = 0;
        let mut best_uct = f32::NEG_INFINITY;
        let mut worst_score = f32::INFINITY;
        let ln_n = (self.stats.visits as f32).ln();

        for (i, child) in self.children.iter().enumerate() {
            let uct = child.uct(ln_n, &mut args.rng);
            if uct > best_uct {
                best_child_index = i;
                best_uct = uct;
            }

            if child.score() < worst_score {
                worst_score = child.score();
            }
        }

        if best_uct > worst_score {
            (false, best_child_index)
        } else {
            self.make_child(args)
        }
    }

    pub fn make_child(&mut self, args: &SearchArgs<'a>) -> (bool, usize) {
        let general_next = self.general_node.borrow_mut().explore(args);
        let boards_next = self.board_nodes.iter()
                .map(|node| node.borrow_mut().explore(args))
                .collect::<StdVec<_>>();

        (true, 0)
    }

    pub fn score(&self) -> f32 {
        self.stats.eval.winprob(self.state.side_to_move)
    }

    pub fn uct(&self, ln_n: f32, rng: &mut impl Rng) -> f32 {
        const C: f32 = 1.5;
        const EPS: f32 = 0.1;

        let w = self.score();
        let n0 = self.stats.visits as f32;
        let r = rng.gen_range(-EPS..EPS);

        w + C * (ln_n / (n0 + 1.0)).sqrt() + r
    }

    pub fn eval(&self) -> Eval {
        self.stats.eval
    }

    pub fn best_turn(&self) -> Turn {
        todo!()
    }
}