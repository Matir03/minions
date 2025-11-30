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

use std::rc::Rc;

pub type CombinedEnc<'a> = Rc<GameState<'a>>;

pub struct NaiveHeuristic<'a> {
    pub config: &'a GameConfig,
}

impl<'a> Heuristic<'a> for NaiveHeuristic<'a> {
    type CombinedEnc = CombinedEnc<'a>;
    type BlottoGen = NaiveBlotto;

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
        NaiveBlotto {
            total_money: combined.money[combined.side_to_move],
            num_boards: combined.boards.len(),
        }
    }

    fn compute_eval(&self, _: &<Self as Heuristic<'a>>::CombinedEnc) -> Eval {
        todo!()
    }
}
