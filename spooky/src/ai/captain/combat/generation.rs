use crate::ai::captain::combat::{
    graph::CombatGraph,
    manager::CombatManager,
    prophet::{DeathProphet, RemovalAssumption},
};
use crate::core::{board::Board, side::Side};
use anyhow::Result;
use z3::ast::Bool;

/// Combat generation system that orchestrates the new architecture
pub struct CombatGenerationSystem {
    pub death_prophet: DeathProphet,
}

impl CombatGenerationSystem {
    pub fn new() -> Self {
        Self {
            death_prophet: DeathProphet::new(crate::ai::rng::make_rng()),
        }
    }

    /// Generate combat actions using the new architecture
    pub fn generate_combat<'ctx>(
        &mut self,
        manager: &mut CombatManager<'ctx>,
        board: &Board,
        side: Side,
    ) -> Result<()> {
        // Get assumptions from the death prophet
        let assumptions = self.death_prophet.generate_assumptions(board, side);

        // Try to add assumptions one by one until solver returns unsat
        let mut max_satisfiable_prefix = 0;
        for (i, assumption) in assumptions.iter().enumerate() {
            // Create constraint for this assumption
            let constraint =
                self.create_assumption_constraint(&manager.variables, assumption, manager.ctx);
            manager.solver.push();
            manager.solver.assert(&constraint);

            // Check if solver is still satisfiable
            match manager.solver.check() {
                z3::SatResult::Sat => {
                    // Assumption is satisfiable, continue
                    max_satisfiable_prefix = i + 1;
                }
                z3::SatResult::Unsat => {
                    // Assumption makes problem unsat, revert and stop
                    manager.solver.pop(1);
                    break;
                }
                z3::SatResult::Unknown => {
                    panic!(
                        "Unknown result from SAT solver: {:?}",
                        manager.solver.get_reason_unknown().unwrap()
                    );
                }
            }
        }

        // Give feedback to death prophet
        self.death_prophet.receive_feedback(max_satisfiable_prefix);

        Ok(())
    }

    /// Create a Z3 constraint for a removal assumption
    fn create_assumption_constraint<'ctx>(
        &self,
        variables: &crate::ai::captain::combat::manager::SatVariables<'ctx>,
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
        let _system = CombatGenerationSystem::new();
        // Should not panic
    }

    #[test]
    fn test_empty_board_generates_no_actions() {
        let map = Map::BlackenedShores;
        let board = Board::new(&map);
        let ctx = Context::new(&z3::Config::new());
        let graph = board.combat_graph(Side::Yellow);
        let mut solver = CombatManager::new(&ctx, graph, &board);

        let mut system = CombatGenerationSystem::new();

        let result = system.generate_combat(&mut solver, &board, Side::Yellow);
        assert!(result.is_ok());
    }
}
