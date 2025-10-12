//! Search for the best move

use crate::ai::game_node::GameChildGenArgs;
use crate::core::{GameConfig, GameState, GameTurn};

use crate::ai::{
    eval::Eval,
    game_node::{GameNode, GameNodeRef, GameNodeState},
};
use crate::heuristics::Heuristic;

use crate::utils::make_rng;

use bumpalo::Bump;
use rand::prelude::*;
use std::cell::RefCell;
use std::vec::Vec as StdVec;

pub struct SearchResult {
    pub eval: Eval,
    pub best_turn: GameTurn,
    pub nodes_explored: u32,
}

pub struct SearchTree<'a, H: Heuristic<'a>> {
    pub rng: StdRng,
    pub root: GameNodeRef<'a, H>,
}

impl<'a, H: Heuristic<'a>> SearchTree<'a, H> {
    pub fn new(
        config: &'a GameConfig,
        state: GameState<'a>,
        arena: &'a Bump,
        heuristic: &'a H,
    ) -> Self {
        let side = state.side_to_move;
        let game_node_state = GameNodeState::from_game_state(state, arena, heuristic);
        let root = arena.alloc(RefCell::new(GameNode::new(game_node_state, side, arena)));

        Self {
            rng: make_rng(),
            root,
        }
    }

    pub fn explore(&mut self, config: &'a GameConfig, arena: &'a Bump, heuristic: &'a H) {
        let mut current_mcts_node = self.root;
        let mut explored_nodes = StdVec::<GameNodeRef<'a, H>>::new();
        explored_nodes.push(current_mcts_node);

        loop {
            if current_mcts_node.borrow().is_terminal() {
                break;
            }

            let (is_new, child_idx) = current_mcts_node.borrow_mut().poll(
                arena,
                heuristic,
                &mut self.rng,
                GameChildGenArgs { arena, heuristic },
            );

            current_mcts_node = current_mcts_node.borrow().edges[child_idx].child;
            explored_nodes.push(current_mcts_node);

            if is_new {
                break;
            }
        }

        let leaf_eval = Eval::static_eval(config, &current_mcts_node.borrow().state.game_state);

        for node_ref in explored_nodes {
            let mut node_borrowed = node_ref.borrow_mut();

            node_borrowed.update(&leaf_eval);
            node_borrowed
                .state
                .board_nodes
                .iter()
                .for_each(|board_node| {
                    board_node.borrow_mut().update(&leaf_eval);
                });
            node_borrowed
                .state
                .general_node
                .borrow_mut()
                .update(&leaf_eval);
        }
    }

    pub fn best_turn(&self) -> GameTurn {
        self.root.borrow().best_turn()
    }

    pub fn eval(&self) -> Eval {
        self.root
            .borrow()
            .stats
            .eval
            .expect("Root node has no eval")
    }

    pub fn result(&self) -> SearchResult {
        SearchResult {
            best_turn: self.best_turn(),
            eval: self.eval(),
            nodes_explored: self.root.borrow().stats.visits,
        }
    }
}
