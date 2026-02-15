use crate::{
    ai::Eval,
    core::{
        board::{actions::BoardTurn, Board},
        game::GameConfig,
        tech::{TechAssignment, TechState, SPELL_COST},
        utils::Sigmoid,
        Blotto, GameState, Side,
    },
    heuristics::{traits::BlottoGen, BoardHeuristic, GeneralHeuristic, Heuristic},
};

use super::blotto::SmartBlotto;

use std::rc::Rc;

pub type CombinedEnc<'a> = Rc<GameState<'a>>;

pub struct SmartHeuristic<'a> {
    pub config: &'a GameConfig,
}

impl<'a> Heuristic<'a> for SmartHeuristic<'a> {
    type CombinedEnc = CombinedEnc<'a>;
    type BlottoGen = SmartBlotto;

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
        SmartBlotto {
            total_money: combined.money[combined.side_to_move],
            num_boards: combined.boards.len(),
        }
    }

    fn compute_eval(&self, combined: &CombinedEnc<'a>) -> Eval {
        let state = combined.as_ref();
        if let Some(winner) = state.winner {
            return Eval::new(1.0, winner);
        }

        const BOARD_POINT_VALUE: i32 = 30;
        const SIGMOID_SCALE: f32 = 0.01;
        let have_the_move_bonus = 10 * self.config.num_boards as i32;

        let mut dollar_diff = 0;

        // Dollars on boards
        for board in state.boards.iter() {
            for piece in board.pieces.values() {
                let value = piece.unit.stats().cost;
                dollar_diff += value * piece.side.sign();
            }
        }

        for side in Side::all() {
            // Dollars on techline
            let num_techs = state.tech_state.acquired_techs[side].len() as i32;
            dollar_diff += num_techs * SPELL_COST * self.config.num_boards as i32 * side.sign();

            // Money in hand
            dollar_diff += state.money[side] * side.sign();

            // Board points value
            dollar_diff += state.board_points[side] * BOARD_POINT_VALUE * side.sign();
        }

        // Add a small bonus for the side to move
        dollar_diff += have_the_move_bonus * state.side_to_move.sign();

        // The dollar_diff is from Yellow's perspective. We need to adjust for side_to_move.
        let perspective_diff = dollar_diff * state.side_to_move.sign();

        // Convert to winprob
        let winprob = (perspective_diff as f32 * SIGMOID_SCALE).sigmoid();

        Eval::new(winprob, state.side_to_move)
    }
}
