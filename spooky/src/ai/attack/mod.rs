pub mod combat;
pub mod solver;
mod constraints;

use crate::core::{action::BoardAction, Board, Side};
use solver::CombatSolver;
use z3::Context;

/// The attack stage generates combat moves using constraint satisfaction.
pub struct AttackStage<'ctx> {
    ctx: &'ctx Context,
    // We can store other state here if needed, for example, to avoid recomputing
    // the combat graph if the board state hasn't changed in a way that affects it.
}

impl<'ctx> AttackStage<'ctx> {
    pub fn new(ctx: &'ctx Context) -> Self {
        Self { ctx }
    }

    /// Generate candidate moves for the attack stage.
    pub fn generate_candidates(
        &mut self,
        board: &Board,
        _side: Side,
        num_candidates: usize,
    ) -> Vec<Vec<BoardAction>> {
        let mut candidates = Vec::new();
        let graph = board.combat_graph();

        // If there are no attackers, there's nothing to solve.
        if graph.attackers.is_empty() {
            return vec![vec![]]; // Return a single empty move list.
        }

        let solver = CombatSolver::new(self.ctx, graph, board);

        for _ in 0..num_candidates {
            if let Some(model) = solver.solve() {
                let actions = solver.generate_move_from_model(&model);
                if !actions.is_empty() && !candidates.contains(&actions) {
                    candidates.push(actions);
                }
                // Exclude the found solution to get a new one in the next iteration.
                solver.block_solution(&model);
            } else {
                // No more solutions found.
                break;
            }
        }

        // If no attack plans were found, it means the only valid plan is to do nothing.
        if candidates.is_empty() {
            candidates.push(vec![]);
        }

        candidates
    }
}