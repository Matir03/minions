use crate::{
    ai::Eval,
    core::{
        board::actions::BoardTurn,
        tech::{TechAssignment, TechState, Techline},
        Board, GameState, Map,
    },
};

pub trait NN<'a>: TechlineNN<'a, Self::Shared> + BoardNN<'a, Self::Shared> {
    type Shared;

    fn compute_shared(
        &self,
        game_state: &GameState<'a>,
        general: &<Self as LocalNN<&'a Techline, TechState, TechAssignment, Self::Shared>>::Enc,
        boards: &[&<Self as LocalNN<&'a Map, Board<'a>, BoardTurn, Self::Shared>>::Enc],
    ) -> Self::Shared;

    fn compute_blottos(&self, shared: &Self::Shared) -> Vec<Vec<i32>>;
    fn compute_eval(&self, shared: &Self::Shared) -> Eval;
}

pub trait LocalNN<Config, State, Turn, Shared> {
    type Acc;
    type Enc;
    type Pre;

    fn new(config: Config) -> Self;

    fn compute_acc(&self, state: &State) -> Self::Acc;
    fn update_acc(&self, acc: &Self::Acc, turn: &Turn) -> Self::Acc;

    fn compute_enc(&self, acc: &Self::Acc) -> Self::Enc;
    fn compute_pre(&self, state: &State, enc: &Self::Enc) -> Self::Pre;

    fn compute_turn(&self, blotto: i32, shared: &Shared, pre: &Self::Pre) -> Turn;
}

pub trait TechlineNN<'a, Shared>: LocalNN<&'a Techline, TechState, TechAssignment, Shared> {}
pub trait BoardNN<'a, Shared>: LocalNN<&'a Map, Board<'a>, BoardTurn, Shared> {}
