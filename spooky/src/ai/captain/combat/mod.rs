use z3::{Context, Optimize};
use crate::core::{Board, Side};
use self::solver::CombatSolver;

pub mod combat;
pub mod constraints;
pub mod solver;

pub struct CombatStage<'ctx> {
    ctx: &'ctx Context,
    optimizer: &'ctx Optimize<'ctx>,
}

impl<'ctx> CombatStage<'ctx> {
    pub fn new(ctx: &'ctx Context, optimizer: &'ctx Optimize<'ctx>) -> Self {
        Self { ctx, optimizer }
    }

    /// Adds combat constraints to the optimizer and returns the solver instance.
    pub fn add_constraints(&self, board: &Board, side: Side) -> CombatSolver<'ctx> {
        let graph = board.combat_graph(side);
        CombatSolver::new(self.ctx, self.optimizer, graph, board)
    }
}