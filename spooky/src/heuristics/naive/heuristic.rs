use crate::{
    ai::Eval,
    core::{
        board::{actions::BoardTurn, Board},
        game::GameConfig,
        tech::{TechAssignment, TechState},
        Blotto, GameState,
    },
    heuristics::{traits::BlottoGen, BoardHeuristic, GeneralHeuristic, Heuristic},
};

use super::blotto::NaiveBlotto;

pub type CombinedEnc<'a> = GameState<'a>;

pub struct NaiveHeuristic;

impl<'a> Heuristic<'a> for NaiveHeuristic {
    type CombinedEnc = CombinedEnc<'a>;
    type BlottoGen = NaiveBlotto;

    fn new(config: &'a GameConfig) -> Self {
        Self
    }

    fn compute_combined(
        &self,
        game_state: &GameState<'a>,
        _: &<Self as GeneralHeuristic<'a, Self::CombinedEnc>>::GeneralEnc,
        _: &[&<Self as BoardHeuristic<'a, Self::CombinedEnc>>::BoardEnc],
    ) -> <Self as Heuristic<'a>>::CombinedEnc {
        game_state.clone()
    }

    fn compute_blottos(&self, combined: &GameState<'a>) -> Self::BlottoGen {
        NaiveBlotto {
            total_money: combined.money[combined.side_to_move],
            num_boards: combined.boards.len(),
        }
    }

    fn compute_eval(&self, _: &<Self as Heuristic<'a>>::CombinedEnc) -> Eval {
        todo!()
    }
}
