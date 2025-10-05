use crate::core::{board::actions::BoardTurn, Board, GameConfig};
use crate::heuristics::naive::NaiveHeuristic;
use crate::heuristics::BoardHeuristic;

impl<'a> BoardHeuristic<'a> for NaiveHeuristic {
    type BoardEnc = ();

    fn compute_enc(&self, board: &Board<'a>) -> Self::BoardEnc {
        ()
    }

    fn update_enc(&self, enc: &Self::BoardEnc, turn: &BoardTurn) -> Self::BoardEnc {
        ()
    }
}
