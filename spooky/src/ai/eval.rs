//! Position evaluation for single boards

use crate::core::{tech::SPELL_COST, utils::Sigmoid, GameConfig, GameState, Side, SideArray};

#[derive(Debug, Clone, Copy, Default)]
pub struct Eval {
    pub winprobs: SideArray<f32>,
}

impl Eval {
    pub fn new(winprob: f32, side: Side) -> Self {
        let mut winprobs = SideArray::new(0.0, 0.0);
        winprobs[side] = winprob;
        winprobs[!side] = 1.0 - winprob;
        Self { winprobs }
    }

    pub fn score(&self, side: Side) -> f32 {
        self.winprobs[side]
    }

    pub fn flip(&self) -> Self {
        Self {
            winprobs: SideArray::new(self.winprobs[Side::Blue], self.winprobs[Side::Yellow]),
        }
    }

    pub fn update_weighted(&mut self, n: f32, eval: &Eval) {
        self.winprobs[Side::Yellow] =
            (self.winprobs[Side::Yellow] * n + eval.winprobs[Side::Yellow]) / (n + 1.0);
        self.winprobs[Side::Blue] =
            (self.winprobs[Side::Blue] * n + eval.winprobs[Side::Blue]) / (n + 1.0);
    }
}
