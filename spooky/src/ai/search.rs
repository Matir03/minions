//! Search for the best move

use crate::core::{GameConfig, GameState, Turn};

use super::{
    mcts::{GameNode, MCTSNode},
    eval::Eval,
    rng::make_rng,
};

use bumpalo::Bump;
use rand::prelude::*;

pub struct SearchResult {
    pub best_turn: Turn,
    pub eval: Eval,
}

pub struct SearchArgs<'a> {
    pub config: &'a GameConfig,
    pub arena: &'a Bump,
}

pub struct Search<'a> {
    pub args: SearchArgs<'a>,
    pub rng: StdRng,
    pub root: GameNode<'a>,
}

impl<'a> Search<'a> {
    pub fn new(config: &'a GameConfig, state: GameState, arena: &'a Bump) -> Self {
        // let arena = Bump::new();
        let root = GameNode::from_state(state, config, arena);

        Self {
            args: SearchArgs {
                config,
                arena,
            },
            rng: make_rng(),
            root,
        }
    }

    pub fn explore(&mut self) {
        self.root.explore(&self.args, &mut self.rng);
    }

    pub fn best_turn(&self) -> Turn {
        self.root.best_turn()
    }

    pub fn eval(&self) -> Eval {
        self.root.eval()
    }

    pub fn result(&self) -> SearchResult {
        SearchResult {
            best_turn: self.best_turn(),
            eval: self.eval(),
        }
    }
}
