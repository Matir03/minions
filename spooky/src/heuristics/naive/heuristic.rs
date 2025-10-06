use crate::{
    ai::Eval,
    core::{
        board::{actions::BoardTurn, Board},
        game::GameConfig,
        tech::{TechAssignment, TechState},
        GameState,
    },
    heuristics::{BoardHeuristic, GeneralHeuristic, Heuristic},
};

pub struct NaiveHeuristic {}

impl<'a> Heuristic<'a> for NaiveHeuristic {
    type Shared = ();

    fn new(config: &'a GameConfig) -> Self {
        Self {}
    }

    fn compute_shared(
        &self,
        _: &GameState<'a>,
        _: &<Self as GeneralHeuristic<'a>>::GeneralEnc,
        _: &[&<Self as BoardHeuristic<'a>>::BoardEnc],
    ) -> <Self as Heuristic<'a>>::Shared {
        ()
    }

    fn compute_blottos(&self, _: &<Self as Heuristic<'a>>::Shared) -> Vec<Vec<i32>> {
        todo!()
    }

    fn compute_eval(&self, _: &<Self as Heuristic<'a>>::Shared) -> Eval {
        todo!()
    }

    fn compute_board_turn(
        &self,
        _: i32,
        _: &<Self as Heuristic<'a>>::Shared,
        _: &<Self as BoardHeuristic<'a>>::BoardEnc,
    ) -> BoardTurn {
        todo!()
    }

    fn compute_general_turn(
        &self,
        _: i32,
        _: &<Self as Heuristic<'a>>::Shared,
        _: &<Self as GeneralHeuristic<'a>>::GeneralEnc,
    ) -> TechAssignment {
        todo!()
    }
}
