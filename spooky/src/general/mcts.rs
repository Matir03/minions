//! Strategic decision making across multiple boards

use crate::core::game::{GameConfig, GameState};
use crate::core::action::Turn;
use crate::core::side::Side;

pub struct Eval {
    // TODO
}

impl Eval {
    pub fn new() -> Self {
        todo!()
    }

    pub fn winprob(&self, side: Side) -> f32 {
        todo!()
    }
}

pub struct Node<'g> {
    config: &'g GameConfig,
    state: GameState,
    // TODO
}

impl<'g> Node<'g> {
    pub fn new(config: &'g GameConfig, state: GameState) -> Self {
        Self {
            config,
            state,
        }
    }

    pub fn explore(&mut self) {
        todo!()
    }

    pub fn eval(&self) -> Eval {
        todo!()
    }

    pub fn best_turn(&self) -> Turn {
        todo!()
    }
}
