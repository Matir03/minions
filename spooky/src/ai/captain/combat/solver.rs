use std::collections::{HashMap, HashSet};

use crate::core::{board::Board, loc::Loc, side::Side, units::{Unit, Attack}, board::actions::{AttackAction, SpawnAction}};

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
/// Z3-based constraint solver for combat resolution.
pub struct CombatSolver<'ctx> {
    ctx: &'ctx Context,
    optimizer: &'ctx Optimize<'ctx>,
    pub variables: Variables<'ctx>,
    pub graph: CombatGraph,
}

impl<'ctx> CombatSolver<'ctx> {
    pub fn new(
        ctx: &'ctx Context,
        optimizer: &'ctx Optimize<'ctx>,
        graph: CombatGraph,
        board: &Board,
    ) -> Self {
        let variables = Variables::new(ctx, &graph);
        add_all_constraints(optimizer, ctx, &variables, &graph, board);

        // Optimization:
        // 1. Maximize the number of attacks.
        // 2. Minimize the number of moves.
        let mut num_attacks = Int::from_i64(ctx, 0);
        for attack_var in variables.attacks.values() {
            let is_attacking = attack_var.ite(&Int::from_i64(ctx, 1), &Int::from_i64(ctx, 0));
            num_attacks = Int::add(ctx, &[&num_attacks, &is_attacking]);
        }
        optimizer.maximize(&num_attacks);

        Self {
            ctx,
            optimizer,
            variables,
            graph,
        }
    }
}

/// Add constraints to the optimizer to exclude the solution represented by the given model.
pub fn block_solution<'ctx>(
    ctx: &'ctx Context,
    optimizer: &Optimize<'ctx>,
    model: &Model<'ctx>,
    variables: &Variables<'ctx>,
) {
    let mut conjuncts: Vec<Bool<'ctx>> = Vec::new();
    for var in variables.move_hex.values() {
        let model_val = model.eval(var, true).unwrap();
        conjuncts.push(var._eq(&model_val));
    }
    for var in variables.attacks.values() {
        let model_val = model.eval(var, true).unwrap();
        conjuncts.push(var._eq(&model_val));
    }
    let conjuncts_refs: Vec<&Bool> = conjuncts.iter().collect();
    let model_constraint = Bool::and(ctx, &conjuncts_refs);
    optimizer.assert(&model_constraint.not());
}

/// Generate a sequence of AttackActions from a satisfying Z3 model.
pub fn generate_move_from_model<'ctx>(
    model: &Model<'ctx>,
    graph: &CombatGraph,
    variables: &Variables<'ctx>,
) -> Vec<AttackAction> {
    let mut actions = Vec::new();
    let mut moves = HashMap::new();
    let mut attacker_new_positions = HashMap::new();

    // First, collect all moves and new positions from the model.
    for attacker in &graph.friends {
        let z3_hex_var = &variables.move_hex[attacker];
        if let Some(val) = model.eval(z3_hex_var, true).unwrap().as_i64() {
            let to_loc = i64_to_loc(val);
            attacker_new_positions.insert(*attacker, to_loc);
            if *attacker != to_loc {
                moves.insert(*attacker, to_loc);
            }
        }
    }

    // Detect cycles and chains and create corresponding move actions.
    let mut visited = HashSet::new();
    for &start_loc in moves.keys() {
        if visited.contains(&start_loc) {
            continue;
        }

        let mut path = Vec::new();
        let mut current_loc = start_loc;

        // Trace the path. Because destinations are unique, each component is either
        // a simple path or a simple cycle.
        loop {
            if visited.contains(&current_loc) {
                // This indicates we've hit a node from another path, which shouldn't happen
                // with unique destinations. We'll treat the path so far as a chain.
                break;
            }

            path.push(current_loc);
            visited.insert(current_loc);

            if let Some(&next_loc) = moves.get(&current_loc) {
                if next_loc == start_loc {
                    // Cycle detected.
                    actions.push(AttackAction::MoveCyclic { locs: path });
                    path = Vec::new(); // Clear path to indicate it's been handled.
                    break;
                }
                current_loc = next_loc;
            } else {
                // End of a chain.
                break;
            }
        }

        // If the path is not empty, it's a chain that needs to be processed.
        if !path.is_empty() {
            for from in path {
                if let Some(&to) = moves.get(&from) {
                    actions.push(AttackAction::Move { from_loc: from, to_loc: to });
                }
            }
        }
    }

    // Next, generate attack actions from the attackers' new positions.
    for ((attacker, defender), is_attacking_var) in &variables.attacks {
        if model
            .eval(is_attacking_var, true)
            .unwrap()
            .as_bool()
            .unwrap_or(false)
        {
            let attacker_pos = attacker_new_positions.get(attacker).unwrap_or(attacker);
            actions.push(AttackAction::Attack {
                attacker_loc: *attacker_pos,
                target_loc: *defender,
            });
        }
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::board::{Modifiers, Piece, PieceState};
    use crate::core::side::Side;
    use crate::core::units::Unit;
    use std::cell::RefCell;
    use crate::core::map::Map;
    use z3::{Config, SatResult};

    fn create_board_with_pieces(pieces: Vec<(Unit, Side, Loc)>) -> Board {
        let mut board = Board::new(Map::AllLand);
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
        let optimizer = Optimize::new(&ctx);
        let graph = board.combat_graph(Side::S0);

        assert_eq!(graph.friends.len(), 1);
        assert_eq!(graph.defenders.len(), 1);

        let solver = CombatSolver::new(&ctx, &optimizer, graph, &board);
        let model = match optimizer.check(&[]) {
            SatResult::Sat => Some(optimizer.get_model().unwrap()),
            _ => None,
        }
        .expect("Should find a solution");

        let actions = generate_move_from_model(&model, &solver.graph, &solver.variables);

        // Expected action: Rat at (2,0) moves to (1,0) and attacks rat at (0,0).
        let expected_move = AttackAction::Move {
            from_loc: Loc::new(2, 0),
            to_loc: Loc::new(1, 0),
        };
        let expected_attack = AttackAction::Attack {
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
        let optimizer = Optimize::new(&ctx);
        let graph = board.combat_graph(Side::S0);
        println!("Graph: {:#?}", graph);
        let solver = CombatSolver::new(&ctx, &optimizer, graph, &board);
        let assumption = Bool::new_const(&ctx, "kill_defender");
        optimizer.assert(&assumption._eq(
            &Bool::and(&ctx, 
                &[
                    &solver.variables.attacks[&(Loc::new(1, 0), Loc::new(0, 0))], 
                    &solver.variables.move_hex[&Loc::new(1, 0)].
                        _eq(&Int::from_i64(&ctx, loc_to_i64(&Loc::new(1, 0))))
                ]
            )
        ));
        let model = match optimizer.check(&[assumption]) {
            SatResult::Sat => Some(optimizer.get_model().unwrap()),
            _ => None,
        }
        .expect("Should find a solution");
        let actions = generate_move_from_model(&model, &solver.graph, &solver.variables);

        // Expected action: Rat at (1,0) attacks rat at (0,0). No move.
        let expected_attack = AttackAction::Attack {
            attacker_loc: Loc::new(1, 0),
            target_loc: Loc::new(0, 0),
        };

        assert_eq!(actions.len(), 1, "Expected 1 action, just attack");
        assert_eq!(actions[0], expected_attack, "Action should be an attack");
    }
}