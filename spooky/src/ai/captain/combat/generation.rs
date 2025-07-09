use std::collections::HashSet;

use crate::ai::captain::combat::{
    constraints::ConstraintManager,
    constraints::SatVariables,
    graph::CombatGraph,
    prophet::{DeathProphet, RemovalAssumption},
};

use crate::core::{board::Board, side::Side};
use anyhow::{Context, Result};
use z3::ast::{Ast, Bool, BV};

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
        manager: &mut ConstraintManager<'ctx>,
        board: &Board,
        side: Side,
    ) -> Result<()> {
        // Get assumptions from the death prophet
        let assumptions = self.death_prophet.generate_assumptions(board, side);
        let constraints = assumptions
            .iter()
            .map(|assumption| {
                self.create_assumption_constraint(&manager.variables, assumption, manager.ctx)
            })
            .collect::<Vec<_>>();

        let mut constraint_set = constraints.iter().cloned().collect::<HashSet<_>>();
        let mut n = 0;

        for i in 0.. {
            n = i;

            let cur_constraints = constraint_set.iter().cloned().collect::<Vec<_>>();
            let is_sat = manager.solver.check_assumptions(&cur_constraints);

            match is_sat {
                z3::SatResult::Sat => {
                    break;
                }
                z3::SatResult::Unsat => {
                    let unsat_core = manager.solver.get_unsat_core();
                    let unsat_core_set = unsat_core.iter().collect::<HashSet<_>>();

                    let bad_constraint = constraints
                        .iter()
                        .rev()
                        .find(|c| unsat_core_set.contains(c))
                        .unwrap();

                    constraint_set.remove(bad_constraint);
                }
                z3::SatResult::Unknown => {
                    panic!(
                        "Unknown result from SAT solver: {:?}",
                        manager.solver.get_reason_unknown().unwrap()
                    );
                }
            }
        }

        println!("Number of retries: {}", n);

        // Give feedback to death prophet
        let active_constraints = constraints
            .iter()
            .map(|c| constraint_set.contains(c))
            .collect::<Vec<_>>();

        self.death_prophet.receive_feedback(active_constraints);

        Ok(())
    }

    /// Create a Z3 constraint for a removal assumption
    fn create_assumption_constraint<'ctx>(
        &self,
        variables: &'ctx SatVariables<'ctx>,
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
            RemovalAssumption::Move(from_loc, to_loc) => {
                // Constraint: move_hex[from_loc] == to_loc
                let move_hex_var = variables
                    .move_hex
                    .get(from_loc)
                    .expect(&format!("No move_hex variable for location {}", from_loc));
                move_hex_var._eq(&to_loc.as_z3(ctx))
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
        let mut solver = ConstraintManager::new(&ctx, graph, &board);

        let mut system = CombatGenerationSystem::new();

        let result = system.generate_combat(&mut solver, &board, Side::Yellow);
        assert!(result.is_ok());
    }
}
