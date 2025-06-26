//! Search for the best move

use crate::core::{GameConfig, GameState, GameTurn};

use crate::ai::{
    eval::Eval,
    game::{GameNode, GameNodeRef, GameNodeState},
    rng::make_rng,
};

use bumpalo::Bump;
use rand::prelude::*;
use std::vec::Vec as StdVec;
use std::cell::RefCell;

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
        let game_node_state = GameNodeState::from_game_state(state, arena);
        let root = arena.alloc(RefCell::new(GameNode::new(game_node_state, arena)));

        Self {
            args: search_args_for_init,
            rng: make_rng(),
            root: root,
        }
    }

    pub fn explore(&mut self) {
        let mut current_mcts_node = self.root;
        let mut explored_nodes = StdVec::<GameNodeRef<'a>>::new();

        loop {
            explored_nodes.push(current_mcts_node);

            let (is_new, child_idx) = current_mcts_node
                .borrow_mut()
                .poll(&self.args, &mut self.rng, (self.args.clone(), self.args.arena));

            if is_new { break; }

            current_mcts_node = current_mcts_node.borrow().edges[child_idx].child;
        }

        let (leaf_eval, leaf_side) = {
            let leaf_node = current_mcts_node.borrow();
            let eval = Eval::static_eval(self.args.config, &leaf_node.state.game_state);
            let side = leaf_node.state.game_state.side_to_move;
            (eval, side)
        };

        for node_ref in explored_nodes {
            let mut node_borrowed = node_ref.borrow_mut();
            let perspective_eval = if node_borrowed.state.game_state.side_to_move != leaf_side {
                leaf_eval.flip()
            } else {
                leaf_eval
            };
            
            node_borrowed.update(&perspective_eval);
            node_borrowed.state.board_nodes.iter().for_each(|board_node| {
                board_node.borrow_mut().update(&perspective_eval);
            });
            node_borrowed.state.general_node.borrow_mut().update(&perspective_eval);
        }
    }

    pub fn best_turn(&self) -> GameTurn {
        self.root.borrow().construct_best_turn() 
    }

    pub fn eval(&self) -> Eval {
        self.root.borrow().stats.eval
    }

    pub fn result(&self) -> SearchResult {
        SearchResult {
            best_turn: self.best_turn(),
            eval: self.eval(),
            nodes_explored: self.root.borrow().stats.visits,
        }
    }
}
