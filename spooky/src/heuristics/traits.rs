use crate::{
    ai::Eval,
    core::{
        board::actions::BoardTurn,
        tech::{TechAssignment, TechState, Techline},
        Board, GameState, Map,
    },
};

pub trait Heuristic<'a>:
    TechlineHeuristic<'a, Self::Shared> + BoardHeuristic<'a, Self::Shared>
{
    type Shared;

    fn compute_shared(
        &self,
        game_state: &GameState<'a>,
        general: &<Self as LocalHeuristic<&'a Techline, TechState, TechAssignment, Self::Shared>>::Acc,
        boards: &[&<Self as LocalHeuristic<&'a Map, Board<'a>, BoardTurn, Self::Shared>>::Acc],
    ) -> Self::Shared;

    fn compute_blottos(&self, shared: &Self::Shared) -> Vec<Vec<i32>>;
    fn compute_eval(&self, shared: &Self::Shared) -> Eval;
}

pub trait LocalHeuristic<Config, State, Turn, Shared> {
    type Acc;
    type Pre;

    fn new(config: Config) -> Self;

    fn compute_acc(&self, state: &State) -> Self::Acc;
    fn update_acc(&self, acc: &Self::Acc, turn: &Turn) -> Self::Acc;

    fn compute_pre(&self, state: &State, acc: &Self::Acc) -> Self::Pre;

    fn compute_turn(&self, blotto: i32, shared: &Shared, pre: &Self::Pre) -> Turn;
}

pub trait TechlineHeuristic<'a, Shared>:
    LocalHeuristic<&'a Techline, TechState, TechAssignment, Shared>
{
}
pub trait BoardHeuristic<'a, Shared>:
    LocalHeuristic<&'a Map, Board<'a>, BoardTurn, Shared>
{
}
