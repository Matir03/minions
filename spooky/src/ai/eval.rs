//! Position evaluation for single boards

use crate::core::{GameState, GameConfig, Side, ToIndex};

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
        Self { winprob: 1.0 - self.winprob }
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
        // Constants from the documentation
        const C_H: f32 = 12.0;
        const C_W: f32 = 25.0;
        const C_M: f32 = 1.0;
        const C_B: f32 = 1.0;
        const C_D: f32 = 0.05;

        // C_T depends on number of boards
        let c_t = 4.0 * state.boards.len() as f32;

        // Board points to win
        let w0 = state.board_points[Side::S0] as f32;
        let w1 = state.board_points[Side::S1] as f32;

        // Money
        let m0 = state.money[Side::S0] as f32;
        let m1 = state.money[Side::S1] as f32;

        // Tech score
        let tech_state = &state.tech_state;
        let a0 = tech_state.unlock_index[Side::S0] as f32;
        let a1 = tech_state.unlock_index[Side::S1] as f32;
        let max_advancement = a0.max(a1);
        let gamma: f32 = 0.98;

        let mut t0 = 0.0;
        let mut t1 = 0.0;

        // Calculate tech scores for each side
        for tech in tech_state.acquired_techs[Side::S0].iter() {
            t0 += gamma.powf(max_advancement - tech.to_index().unwrap() as f32);
        }
        for tech in tech_state.acquired_techs[Side::S1].iter() {
            t1 += gamma.powf(max_advancement - tech.to_index().unwrap() as f32);
        }

        let tech_score = t0 - t1 + a0 - a1;

        // Board scores
        let mut board_score = 0.0;
        for board in state.boards.iter() {
            let mut board_value = 0.0;
            for piece in board.pieces.values() {
                let value = piece.unit.stats().cost as f32;
                board_value += match piece.side {
                    Side::S0 => value,
                    Side::S1 => -value,
                };
            }
            board_score += board_value;
        }

        let hmb = state.side_to_move.sign() as f32;

        // Final score calculation
        let d = C_W * (w0 - w1) + c_t * tech_score + C_M * (m0 - m1) + C_B * board_score + C_H * hmb;
        let winprob = d.sigmoid() * C_D;

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