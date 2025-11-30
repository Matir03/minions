use crate::ai::captain::combat::prophet::DeathProphet;
use crate::ai::captain::positioning::SatPositioningSystem;
use crate::core::{board::actions::BoardTurn, Board, GameConfig, Side};
use crate::heuristics::naive::{CombinedEnc, NaiveHeuristic};
use crate::heuristics::{BoardHeuristic, BoardPreTurn, Heuristic};
use rand::{Rng, SeedableRng};

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

    fn compute_board_pre_turn(
        &self,
        rng: &mut impl Rng,
        shared: &CombinedEnc<'a>,
        enc: &Self::BoardEnc,
        board: &Board<'a>,
        side: Side,
    ) -> BoardPreTurn {
        let positioning_system = SatPositioningSystem {};
        let graph = board.combat_graph(side);
        let moves = positioning_system.generate_move_candidates(rng, &graph, board, side);

        let prophet_rng = rand::rngs::StdRng::seed_from_u64(rng.next_u64());
        let mut prophet = DeathProphet::new(prophet_rng);
        let removals = prophet.generate_assumptions(board, side);

        BoardPreTurn { moves, removals }
    }
}
