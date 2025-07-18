//! Position evaluation for single boards

use crate::core::{tech::SPELL_COST, GameConfig, GameState, Side};

#[derive(Debug, Clone, Copy, Default)]
pub struct Eval {
    /// winprob in [0, 1] for current player
    pub winprob: f32,
}

impl Eval {
    pub fn new(winprob: f32) -> Self {
        Self { winprob }
    }

    pub fn score(&self, same_side: bool) -> f32 {
        if same_side {
            self.winprob
        } else {
            -self.winprob
        }
    }

    pub fn flip(&self) -> Self {
        Self {
            winprob: 1.0 - self.winprob,
        }
    }

    pub fn winprob(&self) -> f32 {
        self.winprob
    }

    /// evaluate position without calculating moves
    pub fn static_eval(config: &GameConfig, state: &GameState) -> Self {
        Eval::heuristic(config, state)
    }

    /// evaluate position from heuristics
    pub fn heuristic(config: &GameConfig, state: &GameState) -> Self {
        const BOARD_POINT_VALUE: i32 = 30;
        const SIGMOID_SCALE: f32 = 0.01;
        let have_the_move_bonus = 10 * config.num_boards as i32;

        let mut dollar_diff = 0;

        // Dollars on boards
        for board in state.boards.iter() {
            for piece in board.pieces.values() {
                let value = piece.unit.stats().cost;
                dollar_diff += value * piece.side.sign();
            }
        }

        for side in Side::all() {
            // Dollars on techline
            let num_techs = state.tech_state.acquired_techs[side].len() as i32;
            dollar_diff += num_techs * SPELL_COST * config.num_boards as i32 * side.sign();

            // Money in hand
            dollar_diff += state.money[side] * side.sign();

            // Board points value
            dollar_diff += state.board_points[side] * BOARD_POINT_VALUE * side.sign();
        }

        // Add a small bonus for the side to move
        dollar_diff += have_the_move_bonus * state.side_to_move.sign();

        // The dollar_diff is from Yellow's perspective. We need to adjust for side_to_move.
        let perspective_diff = dollar_diff * state.side_to_move.sign();

        // Convert to winprob
        let winprob = (perspective_diff as f32 * SIGMOID_SCALE).sigmoid();

        Eval::new(winprob)
    }
}

trait Sigmoid {
    fn sigmoid(self) -> f32;
}

impl Sigmoid for f32 {
    fn sigmoid(self) -> f32 {
        1.0 / (1.0 + (-self).exp())
    }
}
