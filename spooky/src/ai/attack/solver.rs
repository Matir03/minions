use std::collections::HashMap;

use crate::core::{action::BoardAction, board::Board, loc::Loc};
use z3::{
    ast::{Ast, Bool, Int},
    Context, Model, Optimize, SatResult,
};

use super::combat::CombatGraph;
use super::constraints::{add_all_constraints, Variables};

// A helper to convert Loc to a unique i64 for Z3.
fn loc_to_i64(loc: &Loc) -> i64 {
    (loc.x as i64) << 32 | (loc.y as i64 & 0xFFFFFFFF)
}

// A helper to convert i64 from Z3 back to Loc.
fn i64_to_loc(val: i64) -> Loc {
    Loc::new((val >> 32) as i32, (val & 0xFFFFFFFF) as i32)
}

/// Z3-based constraint solver for combat resolution.
pub struct CombatSolver<'ctx> {
    ctx: &'ctx Context,
    optimizer: Optimize<'ctx>,
    variables: Variables<'ctx>,
    graph: CombatGraph,
}

impl<'ctx> CombatSolver<'ctx> {
    pub fn new(ctx: &'ctx Context, graph: CombatGraph, board: &Board) -> Self {
        let optimizer = Optimize::new(ctx);
        // Note: The z3::Optimize object does not have a `set_params` method like z3::Solver.
        // A timeout can be passed to `check_with_assumptions_and_params` if needed, but for now
        // we rely on the default behavior.

        let variables = Variables::new(ctx, &graph);
        add_all_constraints(&optimizer, ctx, &variables, &graph, board);

        // Optimization:
        // 1. Maximize the number of attacks.
        // 2. Minimize the number of moves.
        let mut num_attacks = Int::from_i64(ctx, 0);
        for attack_var in variables.attacks.values() {
            let is_attacking = attack_var.ite(&Int::from_i64(ctx, 1), &Int::from_i64(ctx, 0));
            num_attacks = Int::add(ctx, &[&num_attacks, &is_attacking]);
        }
        optimizer.maximize(&num_attacks);

        let mut num_moves = Int::from_i64(ctx, 0);
        let no_hex_val = Int::from_i64(ctx, -1);
        for attacker in &graph.attackers {
            let attacker_hex_var = &variables.attack_hex[attacker];
            let attacker_loc_val = Int::from_i64(ctx, loc_to_i64(attacker));

            let is_not_passive = attacker_hex_var._eq(&no_hex_val).not();
            let is_not_stationary = attacker_hex_var._eq(&attacker_loc_val).not();

            let has_moved = Bool::and(ctx, &[&is_not_passive, &is_not_stationary]);
            num_moves = Int::add(
                ctx,
                &[&num_moves, &has_moved.ite(&Int::from_i64(ctx, 1), &Int::from_i64(ctx, 0))],
            );
        }
        optimizer.minimize(&num_moves);

        Self {
            ctx,
            optimizer,
            variables,
            graph,
        }
    }

    /// Check if the current constraints are satisfiable and return a model if so.
    pub fn solve(&self) -> Option<Model<'ctx>> {
        match self.optimizer.check(&[]) {
            SatResult::Sat => Some(self.optimizer.get_model().unwrap()),
            _ => None,
        }
    }

    /// Add constraints to the optimizer to exclude the solution represented by the given model.
    pub fn block_solution(&self, model: &Model<'ctx>) {
        let mut conjuncts: Vec<Bool<'ctx>> = Vec::new();
        for var in self.variables.passive.values() {
            let model_val = model.eval(var, true).unwrap();
            conjuncts.push(var._eq(&model_val));
        }
        for var in self.variables.attack_hex.values() {
            let model_val = model.eval(var, true).unwrap();
            conjuncts.push(var._eq(&model_val));
        }
        for var in self.variables.attacks.values() {
            let model_val = model.eval(var, true).unwrap();
            conjuncts.push(var._eq(&model_val));
        }
        let conjuncts_refs: Vec<&Bool> = conjuncts.iter().collect();
        let model_constraint = Bool::and(self.ctx, &conjuncts_refs);
        self.optimizer.assert(&model_constraint.not());
    }

    /// Generate a sequence of BoardActions from a satisfying Z3 model.
    pub fn generate_move_from_model(&self, model: &Model<'ctx>) -> Vec<BoardAction> {
        let mut actions = Vec::new();
        let mut attacker_new_positions = HashMap::new();

        // First, determine the new position of each attacker.
        for attacker in &self.graph.attackers {
            let z3_hex_var = &self.variables.attack_hex[attacker];
            if let Some(val) = model.eval(z3_hex_var, true).unwrap().as_i64() {
                if val != -1 {
                    // -1 indicates the attacker is passive and doesn't move.
                    let to_loc = i64_to_loc(val);
                    if *attacker != to_loc {
                        actions.push(BoardAction::Move {
                            from_loc: *attacker,
                            to_loc,
                        });
                    }
                    attacker_new_positions.insert(*attacker, to_loc);
                }
            }
        }

        // Next, generate attack actions from the attackers' new positions.
        for ((attacker, defender), is_attacking_var) in &self.variables.attacks {
            if model
                .eval(is_attacking_var, true)
                .unwrap()
                .as_bool()
                .unwrap_or(false)
            {
                let attacker_pos = attacker_new_positions.get(attacker).unwrap_or(attacker);
                actions.push(BoardAction::Attack {
                    attacker_loc: *attacker_pos,
                    target_loc: *defender,
                });
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::board::{Modifiers, Piece, PieceState};
    use crate::core::side::Side;
    use crate::core::units::Unit;
    use std::cell::RefCell;
    use z3::Config;

    fn create_board_with_pieces(pieces: Vec<(Unit, Side, Loc)>) -> Board {
        let mut board = Board::new();
        for (unit, side, loc) in pieces {
            let piece = Piece {
                unit,
                side,
                loc,
                modifiers: Modifiers::default(),
                state: RefCell::new(PieceState::default()),
            };
            board.add_piece(piece);
        }
        board
    }

    #[test]
    fn test_simple_attack_can_reach() {
        // Attacker Rat at (2,0), Defender Rat at (0,0).
        // Rat has move=1, range=1. It can move to (1,0) and then attack (0,0).
        let board = create_board_with_pieces(vec![
            (Unit::Rat, Side::S0, Loc::new(2, 0)),
            (Unit::Rat, Side::S1, Loc::new(0, 0)),
        ]);

        let z3_cfg = Config::new();
        let ctx = Context::new(&z3_cfg);
        let graph = board.combat_graph();

        assert_eq!(graph.attackers.len(), 1);
        assert_eq!(graph.defenders.len(), 1);

        let solver = CombatSolver::new(&ctx, graph, &board);
        let model = solver.solve().expect("Should find a solution");

        let actions = solver.generate_move_from_model(&model);

        // Expected action: Rat at (2,0) moves to (1,0) and attacks rat at (0,0).
        let expected_move = BoardAction::Move {
            from_loc: Loc::new(2, 0),
            to_loc: Loc::new(1, 0),
        };
        let expected_attack = BoardAction::Attack {
            attacker_loc: Loc::new(1, 0),
            target_loc: Loc::new(0, 0),
        };

        assert_eq!(actions.len(), 2, "Expected 2 actions, move and attack");
        assert!(
            actions.contains(&expected_move),
            "Missing expected move action"
        );
        assert!(
            actions.contains(&expected_attack),
            "Missing expected attack action"
        );
    }

    #[test]
    fn test_simple_attack_no_move_needed() {
        // Attacker Rat at (1,0), Defender Rat at (0,0).
        // Distance is 1, which is within range. No move needed.
        let board = create_board_with_pieces(vec![
            (Unit::Rat, Side::S0, Loc::new(1, 0)),
            (Unit::Rat, Side::S1, Loc::new(0, 0)),
        ]);

        let z3_cfg = Config::new();
        let ctx = Context::new(&z3_cfg);
        let graph = board.combat_graph();
        let solver = CombatSolver::new(&ctx, graph, &board);
        let model = solver.solve().expect("Should find a solution");
        let actions = solver.generate_move_from_model(&model);

        // Expected action: Rat at (1,0) attacks rat at (0,0). No move.
        let expected_attack = BoardAction::Attack {
            attacker_loc: Loc::new(1, 0),
            target_loc: Loc::new(0, 0),
        };

        assert_eq!(actions.len(), 1, "Expected 1 action, just attack");
        assert_eq!(actions[0], expected_attack, "Action should be an attack");
    }
}