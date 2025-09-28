//! Search for the best move

use crate::core::{GameConfig, GameState, GameTurn};

use crate::ai::{
    eval::Eval,
    game::{GameNode, GameNodeRef, GameNodeState},
};
use crate::heuristics::traits::Heuristic;

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

// Generic SearchTree that works with any Heuristic implementation
pub struct SearchTree<'a, H: Heuristic<'a>> {
    pub args: SearchArgs<'a>,
    pub rng: StdRng,
    pub root: GameNodeRef<'a>,
    pub heuristic: H,
}

// Keep the original SearchTree as an alias for backward compatibility
pub type NaiveSearchTree<'a> = SearchTree<'a, crate::heuristics::NaiveHeuristic>;

impl<'a, H: Heuristic<'a>> SearchTree<'a, H> {
    pub fn new(
        config: &'a GameConfig,
        state: GameState<'a>,
        arena: &'a Bump,
        heuristic: H,
    ) -> Self {
        let search_args_for_init = SearchArgs { config, arena };
        let side = state.side_to_move;
        let game_node_state = GameNodeState::from_game_state(state, arena);
        let root = arena.alloc(RefCell::new(GameNode::new(game_node_state, side, arena)));

        Self {
            args: search_args_for_init,
            rng: make_rng(),
            root,
            heuristic,
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

        // Use the heuristic for evaluation instead of hardcoded static_eval
        let leaf_eval = self.evaluate_with_heuristic(&current_mcts_node.borrow().state.game_state);

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

    /// Use the generic heuristic to evaluate a game state
    fn evaluate_with_heuristic(&self, state: &GameState<'a>) -> Eval {
        // Use the heuristic trait to evaluate the position
        // Compute encodings for techline and boards
        use crate::heuristics::traits::LocalHeuristic;

        let tech_acc = <H as LocalHeuristic<
            &'a crate::core::tech::Techline,
            crate::core::tech::TechState,
            crate::core::tech::TechAssignment,
            H::Shared,
        >>::compute_acc(&self.heuristic, &state.tech_state);

        let mut board_accs = Vec::new();
        for board in &state.boards {
            let board_acc = <H as LocalHeuristic<
                &'a crate::core::map::Map,
                crate::core::board::Board<'a>,
                crate::core::board::actions::BoardTurn,
                H::Shared,
            >>::compute_acc(&self.heuristic, board);
            board_accs.push(board_acc);
        }

        let board_acc_refs: Vec<_> = board_accs.iter().collect();

        // Compute shared data and evaluation - Acc is stored with state, reused for eval
        let shared = self
            .heuristic
            .compute_shared(state, &tech_acc, &board_acc_refs);
        self.heuristic.compute_eval(&shared)
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

// Backward compatibility implementation for the original non-generic API
impl<'a> NaiveSearchTree<'a> {
    pub fn new_with_naive_heuristic(
        config: &'a GameConfig,
        state: GameState<'a>,
        arena: &'a Bump,
    ) -> Self {
        let heuristic = crate::heuristics::NaiveHeuristic::new(config);
        SearchTree::new(config, state, arena, heuristic)
    }
}
