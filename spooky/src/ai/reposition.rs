use z3::{
    ast::{Ast, Int},
    Context, Optimize,
};

use crate::{
    ai::combat::{combat::CombatGraph, constraints::Variables},
    core::{board::Board, side::Side},
};

pub struct RepositioningStage<'ctx> {
    ctx: &'ctx Context,
    optimizer: &'ctx Optimize<'ctx>,
}

impl<'ctx> RepositioningStage<'ctx> {
    pub fn new(ctx: &'ctx Context, optimizer: &'ctx Optimize<'ctx>) -> Self {
        Self {
            ctx,
            optimizer,
        }
    }

    /// Adds constraints to encourage non-attacking units to move towards the enemy center of mass.
    pub fn add_constraints(
        &self,
        board: &Board,
        side: Side,
        graph: &CombatGraph,
        variables: &Variables<'ctx>,
    ) {
        let enemy_units: Vec<_> = board
            .pieces
            .values()
            .filter(|p| p.side == !side)
            .collect();
        if enemy_units.is_empty() || graph.friendlies.is_empty() {
            return;
        }

        // 1. Calculate the center of mass of enemy units.
        let mut total_x = 0;
        let mut total_y = 0;
        for enemy in &enemy_units {
            total_x += enemy.loc.x;
            total_y += enemy.loc.y;
        }
        let center_x = Int::from_i64(self.ctx, (total_x / enemy_units.len() as i32) as i64);
        let center_y = Int::from_i64(self.ctx, (total_y / enemy_units.len() as i32) as i64);

        // 2. Create a Z3 objective to minimize the total distance to the center of mass for passive units.
        let mut total_distance = Int::from_i64(self.ctx, 0);
        let power_of_2_32 = Int::from_i64(self.ctx, 4294967296);

        for friendly_loc in &graph.friendlies {
            // A unit is considered non-attacking if it's not part of any attack.
            let attacks_by_this_unit: Vec<_> = graph
                .pairs
                .iter()
                .filter(|p| p.attacker_pos == *friendly_loc)
                .map(|p| &variables.attacks[&(p.attacker_pos, p.defender_pos)])
                .collect();

            let is_not_attacking = if attacks_by_this_unit.is_empty() {
                z3::ast::Bool::from_bool(self.ctx, true)
            } else {
                z3::ast::Bool::or(self.ctx, &attacks_by_this_unit).not()
            };

            let final_hex_var = &variables.attack_hex[friendly_loc];

            // Extract x and y from the 64-bit integer variable.
            let final_x = final_hex_var.div(&power_of_2_32);
            let final_y = final_hex_var.rem(&power_of_2_32);

            let dx = &final_x - &center_x;
            let dist_x = dx.lt(&Int::from_i64(self.ctx, 0)).ite(&dx.unary_minus(), &dx);

            let dy = &final_y - &center_y;
            let dist_y = dy.lt(&Int::from_i64(self.ctx, 0)).ite(&dy.unary_minus(), &dy);
            let manhattan_distance = Int::add(self.ctx, &[&dist_x, &dist_y]);

            // Add `manhattan_distance` to `total_distance` only if the unit is not attacking.
            let distance_if_not_attacking =
                is_not_attacking.ite(&manhattan_distance, &Int::from_i64(self.ctx, 0));
            total_distance = Int::add(self.ctx, &[&total_distance, &distance_if_not_attacking]);
        }

        // 3. Add the minimization objective to the solver.
        self.optimizer.minimize(&total_distance);
    }
}
