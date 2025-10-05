use crate::{
    ai::Eval,
    core::{
        board::actions::BoardTurn,
        tech::{TechAssignment, TechState, Techline},
        Board, GameConfig, GameState, Map,
    },
};

pub trait Heuristic<'a>: GeneralHeuristic<'a> + BoardHeuristic<'a> {
    type Shared: Clone;

    fn new(config: &'a GameConfig) -> Self;

    fn compute_shared(
        &self,
        game_state: &GameState<'a>,
        general: &Self::GeneralEnc,
        boards: &[&Self::BoardEnc],
    ) -> Self::Shared;

    fn compute_blottos(&self, shared: &Self::Shared) -> Vec<Vec<i32>>;
    fn compute_eval(&self, shared: &Self::Shared) -> Eval;

    fn compute_board_turn(
        &self,
        blotto: i32,
        shared: &Self::Shared,
        enc: &Self::BoardEnc,
    ) -> BoardTurn;

    fn compute_general_turn(
        &self,
        blotto: i32,
        shared: &Self::Shared,
        enc: &Self::GeneralEnc,
    ) -> TechAssignment;
}

pub trait GeneralHeuristic<'a>: 'a {
    type GeneralEnc: Clone;

    fn compute_enc(&self, state: &TechState) -> Self::GeneralEnc;
    fn update_enc(&self, enc: &Self::GeneralEnc, turn: &TechAssignment) -> Self::GeneralEnc;
}
pub trait BoardHeuristic<'a>: 'a {
    type BoardEnc: Clone;

    fn compute_enc(&self, state: &Board<'a>) -> Self::BoardEnc;
    fn update_enc(&self, enc: &Self::BoardEnc, turn: &BoardTurn) -> Self::BoardEnc;
}
