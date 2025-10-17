use crate::{
    ai::Eval,
    core::{
        board::actions::BoardTurn,
        tech::{TechAssignment, TechState, Techline},
        Blotto, Board, GameConfig, GameState, Map,
    },
};

pub trait BlottoGen<'a> {
    fn blotto(&self, money_for_spells: i32) -> Blotto;
}

pub trait Heuristic<'a>:
    GeneralHeuristic<'a, Self::CombinedEnc> + BoardHeuristic<'a, Self::CombinedEnc>
{
    type CombinedEnc: Clone;
    type BlottoGen: BlottoGen<'a>;

    fn new(config: &'a GameConfig) -> Self;

    fn compute_combined(
        &self,
        game_state: &GameState<'a>,
        general: &Self::GeneralEnc,
        boards: &[&Self::BoardEnc],
    ) -> Self::CombinedEnc;

    fn compute_blottos(&self, shared: &Self::CombinedEnc) -> Self::BlottoGen;
    fn compute_eval(&self, shared: &Self::CombinedEnc) -> Eval;
}

pub trait GeneralHeuristic<'a, CombinedEnc>: 'a {
    type GeneralEnc;

    fn compute_enc(&self, state: &TechState) -> Self::GeneralEnc;
    fn update_enc(&self, enc: &Self::GeneralEnc, turn: &TechAssignment) -> Self::GeneralEnc;

    fn compute_general_turn(
        &self,
        blotto: i32,
        shared: &CombinedEnc,
        enc: &Self::GeneralEnc,
    ) -> TechAssignment;
}
pub trait BoardHeuristic<'a, CombinedEnc>: 'a {
    type BoardEnc: Clone;

    fn compute_enc(&self, state: &Board<'a>) -> Self::BoardEnc;
    fn update_enc(&self, enc: &Self::BoardEnc, turn: &BoardTurn) -> Self::BoardEnc;

    fn compute_board_turn(
        &self,
        blotto: i32,
        shared: &CombinedEnc,
        enc: &Self::BoardEnc,
    ) -> BoardTurn;
}
