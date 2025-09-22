//! Search for the best move

use crate::core::{GameConfig, GameState, GameTurn};

use crate::ai::{
    eval::Eval,
    game::{GameNode, GameNodeRef, GameNodeState},
};

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

#[derive(Debug, Clone)]
pub struct SearchArgs<'a> {
    pub config: &'a GameConfig,
    pub arena: &'a Bump,
}

pub struct SearchTree<'a> {
    pub args: SearchArgs<'a>,
    pub rng: StdRng,
    pub root: GameNodeRef<'a>,
}

impl<'a> SearchTree<'a> {
    pub fn new(config: &'a GameConfig, state: GameState<'a>, arena: &'a Bump) -> Self {
        // let arena = Bump::new();
        let search_args_for_init = SearchArgs { config, arena };
        let side = state.side_to_move;
        let game_node_state = GameNodeState::from_game_state(state, arena);
        let root = arena.alloc(RefCell::new(GameNode::new(game_node_state, side, arena)));

        Self {
            args: search_args_for_init,
            rng: make_rng(),
            root,
        }
    }

    pub fn explore(&mut self) {
        let mut current_mcts_node = self.root;
        let mut explored_nodes = StdVec::<GameNodeRef<'a>>::new();
        explored_nodes.push(current_mcts_node);

        loop {
            if current_mcts_node.borrow().is_terminal() {
                break;
            }

            let (is_new, child_idx) = current_mcts_node.borrow_mut().poll(
                &self.args,
                &mut self.rng,
                (self.args.clone(), self.args.arena),
            );

            current_mcts_node = current_mcts_node.borrow().edges[child_idx].child;
            explored_nodes.push(current_mcts_node);

            if is_new {
                break;
            }
        }

        let leaf_eval = Eval::static_eval(
            self.args.config,
            &current_mcts_node.borrow().state.game_state,
        );

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
