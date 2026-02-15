use crate::{
    ai::Eval,
    core::{game::GameConfig, GameState},
    heuristics::{traits::BlottoGen, BoardHeuristic, GeneralHeuristic, Heuristic},
};

use super::blotto::RandomBlotto;

use rand::prelude::*;
use std::rc::Rc;

pub type CombinedEnc<'a> = Rc<GameState<'a>>;

pub struct RandomHeuristic<'a> {
    pub config: &'a GameConfig,
}

impl<'a> Heuristic<'a> for RandomHeuristic<'a> {
    type CombinedEnc = CombinedEnc<'a>;
    type BlottoGen = RandomBlotto;

    fn new(config: &'a GameConfig) -> Self {
        Self { config }
    }

    fn compute_combined(
        &self,
        game_state: &GameState<'a>,
        _: &<Self as GeneralHeuristic<'a, Self::CombinedEnc>>::GeneralEnc,
        _: &[&<Self as BoardHeuristic<'a, Self::CombinedEnc>>::BoardEnc],
    ) -> <Self as Heuristic<'a>>::CombinedEnc {
        Rc::new(game_state.clone())
    }

    fn compute_blottos(&self, combined: &CombinedEnc<'a>) -> Self::BlottoGen {
        RandomBlotto {
            total_money: combined.money[combined.side_to_move],
            num_boards: combined.boards.len(),
        }
    }

    fn compute_eval(&self, combined: &CombinedEnc<'a>) -> Eval {
        let state = combined.as_ref();
        if let Some(winner) = state.winner {
            return Eval::new(1.0, winner);
        }

        // Return a random win probability
        let mut rng = rand::rng();
        let winprob = rng.random_range(0.0..1.0);

        Eval::new(winprob, state.side_to_move)
    }
}
