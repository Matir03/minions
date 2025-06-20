//! Search for the best move

use crate::core::{GameConfig, GameState, GameTurn};

use super::{
    game::{GameNode, GameNodeState, GameNodeRef},
    rng::make_rng,
    eval::Eval,
};

use bumpalo::Bump;
use rand::prelude::*;
use std::vec::Vec as StdVec;
use std::cell::RefCell;

pub struct SearchResult {
    pub best_turn: GameTurn,
    pub eval: Eval,
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
    pub fn new(config: &'a GameConfig, state: GameState, arena: &'a Bump) -> Self {
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
        let current_mcts_node = &mut self.root;
        let mut explored_nodes = StdVec::<GameNodeRef<'a>>::new();

        loop {
            explored_nodes.push(current_mcts_node);

            let (is_new, child_idx) = current_mcts_node
                .borrow_mut()
                .poll(&self.args, &mut self.rng, (self.args.clone(), self.args.arena));

            if is_new { break; }

            let current_mcts_node = 
                &mut current_mcts_node.borrow_mut().edges[child_idx].child;
        }

        let leaf_eval = Eval::static_eval(self.args.config, &current_mcts_node.borrow().state.game_state);

        for node in explored_nodes {
            let mut node_borrowed = node.borrow_mut();
            node_borrowed.update(&leaf_eval);
            node_borrowed.state.board_mcts_nodes.iter().for_each(|board_mcts_node| {
                board_mcts_node.borrow_mut().update(&leaf_eval);
            });
            node_borrowed.state.general_mcts_node.borrow_mut().update(&leaf_eval); 
        }
    }

    pub fn best_turn(&self) -> GameTurn {
        self.root.borrow().best_turn() 
    }

    pub fn eval(&self) -> Eval {
        self.root.borrow().stats.eval
    }

    pub fn result(&self) -> SearchResult {
        SearchResult {
            best_turn: self.best_turn(),
            eval: self.eval(),
        }
    }
}
