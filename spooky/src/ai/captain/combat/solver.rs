use std::collections::{HashMap, HashSet};

use crate::core::{board::Board, loc::Loc, side::Side, units::{Unit, Attack}, board::actions::{AttackAction, SpawnAction}};

use z3::{
    ast::{Ast, Bool, Int},
    Context, Model, Optimize, SatResult,
};

use super::combat::CombatGraph;
use super::constraints::{add_all_constraints, Variables};

// A helper to convert i64 from Z3 back to Loc.
fn u64_to_loc(val: u64) -> Loc {
    Loc::new((val >> 4) as i32, (val & 0xF) as i32)
}

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
    board: &Board,
) -> Vec<AttackAction> {
    let mut actions = Vec::new();
    let mut moves = HashMap::new();
    let mut move_times = HashMap::new();
    let mut attacks = HashMap::new();

    // First, collect all moves and new positions from the model.
    for friend in &graph.friends {
        let z3_hex_var = &variables.move_hex[friend];
        let val = model.eval(z3_hex_var, true).unwrap()
            .as_u64().unwrap();

        let to_loc = u64_to_loc(val);
        moves.insert(*friend, to_loc);

        let z3_time_var = &variables.move_time[friend];
        let val = model.eval(z3_time_var, true).unwrap()
            .as_u64().unwrap();

        move_times.insert(*friend, val);
    }

    for ((attacker, defender), num_attacks_var) in &variables.num_attacks {
        let val = model.eval(num_attacks_var, true).unwrap()
            .as_u64().unwrap();

        if val > 0 {
            attacks.entry(*attacker)
                .or_insert_with(Vec::new)
                .push((*defender, val));
        }
    }

    let mut movers_by_tick = moves.keys()
        .map(|&mover| (move_times[&mover], mover))
        .collect::<Vec<_>>();

    movers_by_tick.sort_by_key(|&(time, _)| time);

    let mut grouped_movers = Vec::new();

    let mut current_time = None;
    for (time, mover) in movers_by_tick {
        if current_time != Some(time) {
            current_time = Some(time);
            grouped_movers.push(HashSet::new());
        }
        grouped_movers.last_mut().unwrap().insert(mover);
    }

    let mut defender_damage = HashMap::new();
    let mut defenders_dead = HashSet::new();

    for mover_group in grouped_movers {
        let mut visited = HashSet::new();

        for mover in mover_group.clone() {
            if visited.contains(&mover) {
                continue;
            }

            let mut chain_movers = Vec::new();
            let mut current_loc = mover;
            
            loop {
                chain_movers.push(current_loc);
                visited.insert(current_loc);
                let next_loc = *moves.get(&current_loc).unwrap();

                if next_loc == mover {
                    // cycle detected
                    if chain_movers.len() > 1 { // not just a self-move
                        actions.push(AttackAction::MoveCyclic { locs: chain_movers.clone() });
                    }
                    break;
                }

                if !mover_group.contains(&next_loc) || visited.contains(&next_loc) {
                    // end of chain
                    for from_loc in chain_movers.iter().rev().cloned() {
                        let to_loc = *moves.get(&from_loc).unwrap();
                        actions.push(AttackAction::Move { from_loc, to_loc });
                    }
                    break;
                }

                current_loc = next_loc;
            }

            for original_attacker in chain_movers {
                let attack = board.get_piece(&original_attacker)
                    .unwrap()
                    .unit
                    .stats()
                    .attack;

                if let Some(attacks) = attacks.get(&original_attacker) {
                    let attacker_new_loc = moves.get(&original_attacker).unwrap();
                    for (defender, num_attacks) in attacks {
                        if defenders_dead.contains(defender) {
                            continue;
                        }

                        let defender_stats = board.get_piece(defender)
                            .unwrap()
                            .unit
                            .stats();

                        let dmg_val = match attack {
                            Attack::Damage(damage) => damage,
                            Attack::Unsummon if defender_stats.persistent => 1,
                            _ => defender_stats.defense,
                        };

                        // println!("{} attacks {} {} times", original_attacker, defender, num_attacks);
                        for _ in 0..*num_attacks {
                            actions.push(AttackAction::Attack { 
                                attacker_loc: *attacker_new_loc, 
                                target_loc: *defender 
                            });


                            defender_damage.entry(*defender)
                                .and_modify(|dmg| *dmg += dmg_val)
                                .or_insert(dmg_val);

                            if defender_damage[defender] >= defender_stats.defense {
                                defenders_dead.insert(*defender);
                                break;
                            }
                        }
                    }
                }
            }
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

    #[test]
    fn test_cyclic_move() {
        // Three rats in a triangle, each wanting to move to the next one's spot.
        let map = Map::AllLand;
        let board = create_board_with_pieces(&map, vec![
            (Unit::Rat, Side::Yellow, Loc::new(0, 0)),
            (Unit::Zombie, Side::Yellow, Loc::new(1, 0)),
            (Unit::Skeleton, Side::Yellow, Loc::new(0, 1)),
        ]);

        let z3_cfg = Config::new();
        let ctx = Context::new(&z3_cfg);
        let optimizer = Optimize::new(&ctx);
        let graph = board.combat_graph(Side::Yellow);
        let solver = CombatSolver::new(&ctx, &optimizer, graph, &board);


        let move1 = solver.variables.move_hex[&Loc::new(0, 0)]._eq(&Loc::new(1, 0).as_z3(&ctx));
        let move2 = solver.variables.move_hex[&Loc::new(1, 0)]._eq(&Loc::new(0, 1).as_z3(&ctx));
        let move3 = solver.variables.move_hex[&Loc::new(0, 1)]._eq(&Loc::new(0, 0).as_z3(&ctx));
        optimizer.assert(&move1);
        optimizer.assert(&move2);
        optimizer.assert(&move3);

        let model = match optimizer.check(&[]) {
            SatResult::Sat => Some(optimizer.get_model().unwrap()),
            _ => None,
        }
        .expect("Should find a solution");

        let actions = generate_move_from_model(&model, &solver.graph, &solver.variables, &board);

        assert_eq!(actions.len(), 1, "Expected a single MoveCyclic action");

        // Apply the action and verify the board state
        let mut board_after_move = board.clone();
        let result = board_after_move.do_attack_action(Side::Yellow, actions[0].clone());
        assert!(result.is_ok(), "Cyclic move failed: {:?}", result);

        assert_eq!(board_after_move.get_piece(&Loc::new(0, 0)).unwrap().unit, Unit::Skeleton);
        assert_eq!(board_after_move.get_piece(&Loc::new(1, 0)).unwrap().unit, Unit::Rat);
        assert_eq!(board_after_move.get_piece(&Loc::new(0, 1)).unwrap().unit, Unit::Zombie);

        // Also verify the action itself
        match &actions[0] {
            AttackAction::MoveCyclic { locs } => {
                assert_eq!(locs.len(), 3, "Cycle should involve 3 locations");
                assert!(locs.contains(&Loc::new(0, 0)));
                assert!(locs.contains(&Loc::new(1, 0)));
                assert!(locs.contains(&Loc::new(0, 1)));

                // assert cyclic order
                let rat = locs.iter().position(|&loc| loc == Loc::new(0, 0)).unwrap();
                let zombie = locs.iter().position(|&loc| loc == Loc::new(1, 0)).unwrap();
                let skeleton = locs.iter().position(|&loc| loc == Loc::new(0, 1)).unwrap();
                assert_eq!((3 + skeleton - zombie) % 3, 1);
                assert_eq!((3 + rat - skeleton) % 3, 1);
                assert_eq!((3 + zombie - rat) % 3, 1);
            }
            _ => panic!("Expected a MoveCyclic action, got {:?}", actions[0]),
        }
    }

    fn create_board_with_pieces<'a>(map: &'a Map, pieces: Vec<(Unit, Side, Loc)>) -> Board<'a> {
        let mut board = Board::new(map);
        for (unit, side, loc) in pieces {
            let piece = Piece {
                unit,
                side,
                loc,
                modifiers: Modifiers::default(),
                state: PieceState::default(),
            };
            board.add_piece(piece);
        }
        board
    }

    #[test]
    fn test_simple_attack_can_reach() {
        // Attacker Rat at (2,0), Defender Rat at (0,0).
        // Rat has move=1, range=1. It can move to (1,0) and then attack (0,0).
        let map = Map::AllLand;
        let board = create_board_with_pieces(&map, vec![
            (Unit::Rat, Side::Yellow, Loc::new(2, 0)),
            (Unit::Rat, Side::Blue, Loc::new(0, 0)),
        ]);

        let z3_cfg = Config::new();
        let ctx = Context::new(&z3_cfg);
        let optimizer = Optimize::new(&ctx);
        let graph = board.combat_graph(Side::Yellow);

        assert_eq!(graph.friends.len(), 1);
        assert_eq!(graph.defenders.len(), 1);

        let solver = CombatSolver::new(&ctx, &optimizer, graph, &board);
        let model = match optimizer.check(&[]) {
            SatResult::Sat => Some(optimizer.get_model().unwrap()),
            _ => None,
        }
        .expect("Should find a solution");

        let actions = generate_move_from_model(&model, &solver.graph, &solver.variables, &board);

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
        let map = Map::AllLand;
        let board = create_board_with_pieces(&map, vec![
            (Unit::Rat, Side::Yellow, Loc::new(1, 0)),
            (Unit::Rat, Side::Blue, Loc::new(0, 0)),
        ]);

        let z3_cfg = Config::new();
        let ctx = Context::new(&z3_cfg);
        let optimizer = Optimize::new(&ctx);
        let graph = board.combat_graph(Side::Yellow);
        let solver = CombatSolver::new(&ctx, &optimizer, graph, &board);
        let assumption = Bool::new_const(&ctx, "kill_defender");
        optimizer.assert(&assumption._eq(
            &Bool::and(&ctx, 
                &[
                    &solver.variables.attacks[&(Loc::new(1, 0), Loc::new(0, 0))], 
                    &solver.variables.move_hex[&Loc::new(1, 0)].
                        _eq(&Loc::new(1, 0).as_z3(&ctx))
                ]
            )
        ));
        let model = match optimizer.check(&[assumption]) {
            SatResult::Sat => Some(optimizer.get_model().unwrap()),
            _ => None,
        }
        .expect("Should find a solution");
        let actions = generate_move_from_model(&model, &solver.graph, &solver.variables, &board);

        // Expected action: Rat at (1,0) attacks rat at (0,0). No move.
        let expected_attack = AttackAction::Attack {
            attacker_loc: Loc::new(1, 0),
            target_loc: Loc::new(0, 0),
        };

        assert_eq!(actions.len(), 1, "Expected 1 action, just attack");
        assert_eq!(actions[0], expected_attack, "Action should be an attack");
    }
}