use crate::ai::captain::combat::sat_solver::{SatCombatSolver, SatVariables};
use crate::core::{board::Board, Loc, Side};
use rand::prelude::*;
use std::collections::HashMap;
use z3::ast::{Ast, Bool};

/// Represents a potential move with an importance score
#[derive(Debug, Clone)]
pub struct MoveCandidate {
    pub from_loc: Loc,
    pub to_loc: Loc,
    pub importance_score: f64,
}

/// SAT-based piece positioning system
pub struct SatPositioningSystem {
    rng: StdRng,
}

impl SatPositioningSystem {
    pub fn new(rng: StdRng) -> Self {
        Self { rng }
    }

    /// Order all possible moves by their importance score and try to assert them
    pub fn position_pieces<'ctx>(
        &mut self,
        solver: &mut SatCombatSolver<'ctx>,
        board: &Board,
        side: Side,
    ) -> Result<(), String> {
        // Generate all possible moves with importance scores
        let move_candidates = self.generate_move_candidates(board, side);

        // Sort by importance score (highest first)
        let mut sorted_moves = move_candidates;
        sorted_moves.sort_by(|a, b| b.importance_score.partial_cmp(&a.importance_score).unwrap());

        // Track which friendly locations have been processed
        let mut processed_locs = std::collections::HashSet::new();

        // Try to assert each move in order
        for candidate in sorted_moves {
            // Skip if this friendly location has already been processed
            if processed_locs.contains(&candidate.from_loc) {
                continue;
            }

            // Try to assert this move as an additional assumption
            let move_constraint = self.create_move_constraint(
                &solver.variables,
                candidate.from_loc,
                candidate.to_loc,
                solver.ctx,
            );

            solver.push();
            solver.assert(&move_constraint);

            // Check if the solver is still satisfiable
            match solver.check() {
                z3::SatResult::Sat => {
                    // Move is valid, make this assertion permanent
                    processed_locs.insert(candidate.from_loc);
                    // Note: The assertion is already added to the solver
                }
                z3::SatResult::Unsat => {
                    // Move is invalid, revert this assertion
                    solver.pop(1);
                }
                z3::SatResult::Unknown => {
                    // Timeout or other issue, revert and continue
                    solver.pop(1);
                }
            }
        }

        Ok(())
    }

    /// Generate all possible moves with random importance scores
    fn generate_move_candidates(&mut self, board: &Board, side: Side) -> Vec<MoveCandidate> {
        let mut candidates = Vec::new();

        // Get all friendly pieces
        for (loc, piece) in &board.pieces {
            if piece.side == side {
                // Get all possible move destinations for this piece
                let valid_moves = board.get_theoretical_move_hexes(*loc);

                for to_loc in valid_moves {
                    // Generate a random importance score between 0 and 1
                    let importance_score = self.rng.gen_range(0.0..1.0);

                    candidates.push(MoveCandidate {
                        from_loc: *loc,
                        to_loc,
                        importance_score,
                    });
                }
            }
        }

        candidates
    }

    /// Create a Z3 constraint for a specific move
    fn create_move_constraint<'ctx>(
        &self,
        variables: &SatVariables<'ctx>,
        from_loc: Loc,
        to_loc: Loc,
        ctx: &'ctx z3::Context,
    ) -> Bool<'ctx> {
        // Constraint: move_hex[from_loc] == to_loc
        variables.move_hex[&from_loc]._eq(&to_loc.as_z3(ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::rng::make_rng;
    use crate::core::{board::Board, map::Map, side::Side, units::Unit};
    use z3::Context;

    #[test]
    fn test_generate_move_candidates() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);

        // Add a friendly piece
        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            crate::core::loc::Loc::new(1, 1),
        ));

        let mut positioning = SatPositioningSystem::new(make_rng());
        let candidates = positioning.generate_move_candidates(&board, Side::Yellow);

        // Should generate some move candidates
        assert!(!candidates.is_empty());

        // All candidates should be for the friendly piece
        for candidate in &candidates {
            assert_eq!(candidate.from_loc, crate::core::loc::Loc::new(1, 1));
            assert!(candidate.importance_score >= 0.0);
            assert!(candidate.importance_score < 1.0);
        }
    }

    #[test]
    fn test_candidates_are_sorted_by_importance() {
        let map = Map::BlackenedShores;
        let board = Board::new(&map);

        let mut positioning = SatPositioningSystem::new(make_rng());
        let mut candidates = positioning.generate_move_candidates(&board, Side::Yellow);

        // Sort by importance score
        candidates.sort_by(|a, b| b.importance_score.partial_cmp(&a.importance_score).unwrap());

        // Check that they are in descending order
        for i in 1..candidates.len() {
            assert!(candidates[i - 1].importance_score >= candidates[i].importance_score);
        }
    }
}
