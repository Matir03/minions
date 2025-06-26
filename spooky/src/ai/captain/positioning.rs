use z3::{
    ast::{Ast, Int},
    Context, Optimize,
};

use crate::{
    ai::captain::combat::{combat::CombatGraph, constraints::Variables},
    core::{board::Board, side::Side},
};

pub struct PositioningStage<'ctx> {
    ctx: &'ctx Context,
    optimizer: &'ctx Optimize<'ctx>,
}

impl<'ctx> PositioningStage<'ctx> {
    pub fn new(ctx: &'ctx Context, optimizer: &'ctx Optimize<'ctx>) -> Self {
        Self {
            ctx,
            optimizer,
        }
    }

    /// Adds constraints to encourage movement
    pub fn add_constraints(
        &self,
        board: &Board,
        side: Side,
        graph: &CombatGraph,
        variables: &Variables<'ctx>,
    ) {
        let mut num_movers = Int::from_i64(self.ctx, 0);
        for friend in &graph.friends {
            let is_moving = variables.move_hex[friend]._eq(&friend.as_z3(self.ctx)).not();
            num_movers = Int::add(self.ctx, &[&num_movers, &is_moving.ite(&Int::from_i64(self.ctx, 1), &Int::from_i64(self.ctx, 0))]);
        }
        self.optimizer.maximize(&num_movers);
    }
}
