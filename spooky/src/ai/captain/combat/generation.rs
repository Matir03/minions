use crate::ai::captain::combat::{
    combat::CombatGraph,
    death_prophet::{DeathProphet, RemovalAssumption},
    sat_solver::{generate_move_from_sat_model, SatCombatSolver},
};
use crate::ai::captain::positioning_sat::SatPositioningSystem;
use crate::core::{board::actions::AttackAction, board::Board, side::Side};
use std::collections::HashMap;
use z3::ast::Bool;

/// Combat generation system that orchestrates the new architecture
pub struct CombatGenerationSystem {
    pub death_prophet: DeathProphet,
    positioning_system: SatPositioningSystem,
}

impl CombatGenerationSystem {
    pub fn new(death_prophet: DeathProphet, positioning_system: SatPositioningSystem) -> Self {
        Self {
            death_prophet,
            positioning_system,
        }
    }

    /// Generate combat actions using the new architecture
    pub fn generate_combat<'ctx>(
        &mut self,
        solver: &mut SatCombatSolver<'ctx>,
        board: &Board,
        side: Side,
    ) -> Result<(), String> {
        // Get assumptions from the death prophet
        let assumptions = self.death_prophet.generate_assumptions(board, side);

        // Try to add assumptions one by one until solver returns unsat
        let mut max_satisfiable_prefix = 0;
        for (i, assumption) in assumptions.iter().enumerate() {
            // Create constraint for this assumption
            let constraint =
                self.create_assumption_constraint(&solver.variables, assumption, solver.ctx);
            solver.push();
            solver.assert(&constraint);

            // Check if solver is still satisfiable
            match solver.check() {
                z3::SatResult::Sat => {
                    // Assumption is satisfiable, continue
                    max_satisfiable_prefix = i + 1;
                }
                z3::SatResult::Unsat => {
                    // Assumption makes problem unsat, revert and stop
                    solver.pop(1);
                    break;
                }
                z3::SatResult::Unknown => {
                    // Timeout, revert and stop
                    solver.pop(1);
                    break;
                }
            }
        }

        // Give feedback to death prophet
        self.death_prophet.receive_feedback(max_satisfiable_prefix);

        Ok(())
    }

    /// Position pieces using the importance heuristic
    pub fn position_pieces<'ctx>(
        &mut self,
        solver: &mut SatCombatSolver<'ctx>,
        board: &Board,
        side: Side,
    ) -> Result<(), String> {
        self.positioning_system.position_pieces(solver, board, side)
    }

    /// Re-apply satisfiable assumptions to the solver
    pub fn reapply_satisfiable_assumptions<'ctx>(
        &mut self,
        solver: &mut SatCombatSolver<'ctx>,
        board: &Board,
        side: Side,
    ) -> usize {
        let assumptions = self.death_prophet.generate_assumptions(board, side);
        let mut max_satisfiable_prefix = 0;

        for (i, assumption) in assumptions.iter().enumerate() {
            let constraint =
                self.create_assumption_constraint(&solver.variables, assumption, solver.ctx);
            solver.assert(&constraint);

            match solver.check() {
                z3::SatResult::Sat => {
                    max_satisfiable_prefix = i + 1;
                }
                z3::SatResult::Unsat => {
                    solver.pop(1);
                    break;
                }
                z3::SatResult::Unknown => {
                    solver.pop(1);
                    break;
                }
            }
        }

        max_satisfiable_prefix
    }

    /// Create a Z3 constraint for a removal assumption
    fn create_assumption_constraint<'ctx>(
        &self,
        variables: &crate::ai::captain::combat::sat_solver::SatVariables<'ctx>,
        assumption: &RemovalAssumption,
        ctx: &'ctx z3::Context,
    ) -> Bool<'ctx> {
        match assumption {
            RemovalAssumption::Removed(loc) => {
                // Constraint: removed[loc] == true
                variables
                    .removed
                    .get(loc)
                    .expect(&format!("No removed variable for location {}", loc))
                    .clone()
            }
            RemovalAssumption::NotRemoved(loc) => {
                // Constraint: removed[loc] == false
                variables
                    .removed
                    .get(loc)
                    .expect(&format!("No removed variable for location {}", loc))
                    .not()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::rng::make_rng;
    use crate::core::{board::Board, map::Map, side::Side, units::Unit};
    use z3::Context;

    #[test]
    fn test_combat_generation_system_creation() {
        let death_prophet = DeathProphet::new(crate::ai::rng::make_rng());
        let positioning_system = SatPositioningSystem::new(crate::ai::rng::make_rng());

        let _system = CombatGenerationSystem::new(death_prophet, positioning_system);
        // Should not panic
    }

    #[test]
    fn test_empty_board_generates_no_actions() {
        let map = Map::BlackenedShores;
        let board = Board::new(&map);
        let ctx = Context::new(&z3::Config::new());
        let graph = board.combat_graph(Side::Yellow);
        let mut solver = SatCombatSolver::new(&ctx, graph, &board);

        let death_prophet = DeathProphet::new(crate::ai::rng::make_rng());
        let positioning_system = SatPositioningSystem::new(crate::ai::rng::make_rng());
        let mut system = CombatGenerationSystem::new(death_prophet, positioning_system);

        let result = system.generate_combat(&mut solver, &board, Side::Yellow);
        assert!(result.is_ok());
    }
}
