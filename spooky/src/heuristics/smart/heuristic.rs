use crate::{
    ai::Eval,
    core::{
        board::{actions::BoardTurn, BitboardOps, Board},
        game::GameConfig,
        tech::{TechAssignment, TechState, SPELL_COST},
        utils::Sigmoid,
        Blotto, GameState, Loc, Side,
    },
    heuristics::{
        smart::zones::{HexZone, ZoneAnalysis},
        traits::BlottoGen,
        BoardHeuristic, GeneralHeuristic, Heuristic,
    },
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
        const NECROMANCER_SAFETY_VALUE: f64 = 15.0;
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

        // --- Zone-based evaluation additions ---
        let mut zone_value: f64 = 0.0;

        for board in state.boards.iter() {
            let zone_analysis = ZoneAnalysis::compute(board);

            // Expected discounted graveyard income.
            let graveyard_locs = board.bitboards.graveyards.to_locs();
            for g_loc in &graveyard_locs {
                for side in Side::all() {
                    let is_occupied = board.bitboards.pieces[side].get(*g_loc);
                    if is_occupied {
                        let expected_income =
                            zone_analysis.expected_graveyard_income(*g_loc, side, 1.0);
                        zone_value += expected_income * side.sign() as f64;
                    }
                }
            }

            // Necromancer safety.
            for (loc, piece) in &board.pieces {
                if !piece.unit.stats().necromancer {
                    continue;
                }
                let zone = zone_analysis.zone_at(*loc);
                let safety_multiplier = match zone {
                    HexZone::Protected(s) if s == piece.side => 1.0,
                    HexZone::Covered(s) if s == piece.side => 0.7,
                    HexZone::Contested => 0.3,
                    HexZone::Open => 0.0,
                    HexZone::Covered(s) if s != piece.side => -0.3,
                    HexZone::Protected(s) if s != piece.side => -0.6,
                    _ => 0.0,
                };
                zone_value +=
                    NECROMANCER_SAFETY_VALUE * safety_multiplier * piece.side.sign() as f64;
            }
        }

        // Combine: dollar_diff (integer) + zone_value (float)
        let total_diff = dollar_diff as f64 * state.side_to_move.sign() as f64
            + zone_value * state.side_to_move.sign() as f64;

        // Convert to winprob
        let winprob = (total_diff as f32 * SIGMOID_SCALE).sigmoid();

        Eval::new(winprob, state.side_to_move)
    }
}
