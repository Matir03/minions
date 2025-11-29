use crate::core::{board::actions::BoardTurn, Board, GameConfig};
use crate::heuristics::naive::{CombinedEnc, NaiveHeuristic};
use crate::heuristics::{BoardHeuristic, Heuristic};

impl<'a> BoardHeuristic<'a, CombinedEnc<'a>> for NaiveHeuristic<'a> {
    type BoardEnc = ();

    fn compute_enc(&self, board: &Board<'a>) -> Self::BoardEnc {
        ()
    }

    fn update_enc(&self, enc: &Self::BoardEnc, turn: &BoardTurn) -> Self::BoardEnc {
        ()
    }

    fn compute_board_turn(
        &self,
        blotto: i32,
        shared: &CombinedEnc<'a>,
        enc: &Self::BoardEnc,
    ) -> BoardTurn {
        BoardTurn::default()
    }
}
