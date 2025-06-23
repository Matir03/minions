use z3::{
    ast::{Ast, Int},
    Context, Optimize,
};

use crate::{
    ai::captain::combat::{combat::CombatGraph, constraints::Variables},
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

    /// Adds constraints to encourage non-attacking units to move towards unoccupied graveyards.
    pub fn add_constraints(
        &self,
        board: &Board,
        side: Side,
        graph: &CombatGraph,
        variables: &Variables<'ctx>,
    ) {
        let unoccupied_graveyards = board.unoccupied_graveyards(side);
        if unoccupied_graveyards.is_empty() || graph.friends.is_empty() {
            return;
        }

        let mut total_distance = Int::from_i64(self.ctx, 0);
        let power_of_2_32 = Int::from_i64(self.ctx, 1 << 32);

        for friendly_loc in &graph.friends {
            let attacks_by_this_unit: Vec<_> = graph
                .triples
                .iter()
                .filter(|p| p.attacker_pos == *friendly_loc)
                .map(|p| &variables.attacks[&(p.attacker_pos, p.defender_pos)])
                .collect();

            let is_not_attacking = if attacks_by_this_unit.is_empty() {
                z3::ast::Bool::from_bool(self.ctx, true)
            } else {
                z3::ast::Bool::or(self.ctx, &attacks_by_this_unit).not()
            };

            let final_hex_var = &variables.move_hex[friendly_loc];
            let final_x = final_hex_var.div(&power_of_2_32);
            let final_y = final_hex_var.rem(&power_of_2_32);

            let mut min_dist_to_graveyard = Int::from_i64(self.ctx, i64::MAX);

            for graveyard_loc in &unoccupied_graveyards {
                let gx = Int::from_i64(self.ctx, graveyard_loc.x as i64);
                let gy = Int::from_i64(self.ctx, graveyard_loc.y as i64);

                let dx = &final_x - &gx;
                let dist_x = dx.lt(&Int::from_i64(self.ctx, 0)).ite(&dx.unary_minus(), &dx);

                let dy = &final_y - &gy;
                let dist_y = dy.lt(&Int::from_i64(self.ctx, 0)).ite(&dy.unary_minus(), &dy);
                let manhattan_distance = Int::add(self.ctx, &[&dist_x, &dist_y]);

                min_dist_to_graveyard = min_dist_to_graveyard.lt(&manhattan_distance).ite(&min_dist_to_graveyard, &manhattan_distance);
            }

            let distance_if_not_attacking =
                is_not_attacking.ite(&min_dist_to_graveyard, &Int::from_i64(self.ctx, 0));
            total_distance = Int::add(self.ctx, &[&total_distance, &distance_if_not_attacking]);
        }

        self.optimizer.minimize(&total_distance);
    }
}
