use anyhow::Result;
use rand::prelude::*;
use z3::{Config, Context};

use crate::core::{
    board::{actions::AttackAction, Board},
    side::Side,
};

use super::combat::{attack_solver::AttackSolver, graph::CombatGraph, prophet::DeathProphet};

/// New attack phase system that uses the attack solver strategy
pub struct AttackPhaseSystem {}

impl AttackPhaseSystem {
    pub fn new() -> Self {
        Self {}
    }

    /// Generate attack actions using the new strategy
    pub fn generate_attacks(
        &self,
        board: &Board,
        side: Side,
        rng: &mut impl Rng,
    ) -> Result<Vec<AttackAction>> {
        // Initialize Z3 context
        let z3_cfg = Config::new();
        let ctx = Context::new(&z3_cfg);

        // Create combat graph
        let graph = board.combat_graph(side);

        // Create death prophet
        let death_prophet = DeathProphet::new(StdRng::from_rng(rng).unwrap());

        // Create attack solver
        let mut attack_solver = AttackSolver::new(&ctx, graph, board, death_prophet);

        // Generate attacks using the new strategy
        attack_solver.generate_attacks(board, side)
    }
}

use rand::rngs::StdRng;
