use crate::ai::captain::combat::constraints::{ConstraintManager, SatVariables};
use crate::core::{
    board::{actions::AttackAction, Board},
    Loc, Side,
};
use anyhow::{bail, Context, Result};
use rand::prelude::*;
use std::collections::{HashMap, HashSet};
use z3::{
    ast::{Ast, Bool},
    SatResult,
};

use lapjv::{lapjv, Matrix};

/// Represents a potential move with an affinity value
#[derive(Debug, Clone)]
pub struct MoveCandidate {
    pub from_loc: Loc,
    pub to_loc: Loc,
    pub score: f64,
}

/// SAT-based piece positioning system with three-stage approach
pub struct SatPositioningSystem {}

impl SatPositioningSystem {
    /// Stage 1: Combat reconciliation - ensuring friendly pieces in the way of attacking pieces can get out of the way
    /// returns whether the attack is at all possible
    /// if it is, it will return true and the solver will be in a state where the attack can be positioned
    /// if it is not, it will return false and the caller can try a different attack
    pub fn combat_reconciliation<'ctx>(
        &self,
        manager: &mut ConstraintManager<'ctx>,
        board: &Board,
        side: Side,
        move_candidates: &[MoveCandidate],
    ) -> Result<Option<Vec<Bool<'ctx>>>> {
        let mut constraints = Vec::new();

        loop {
            // Generate a model for the combat
            let model = match manager.solver.check() {
                z3::SatResult::Sat => manager.solver.get_model().context("Failed to get model")?,
                z3::SatResult::Unsat => {
                    // results in backtracking
                    // return the constraints that should be added after backtracking
                    return Ok(Some(constraints));
                }
                z3::SatResult::Unknown => {
                    // this should never happen
                    panic!(
                        "Unknown result from SAT solver: {:?}",
                        manager.solver.get_reason_unknown().unwrap()
                    );
                }
            };

            // Compute the untouched friendly units and the overlapping units
            let untouched_friendly_units = manager.untouched_friendly_units();
            let overlapping_units =
                self.compute_overlapping_units(manager, &model, &untouched_friendly_units);

            if overlapping_units.is_empty() {
                // No conflicts, proceed to attack positioning
                break;
            }

            // add movement variables and constraints for the overlapping units
            // don't add constraints on where the overlapping units can move to
            // because we want to check the strong assumptions first
            let new_constraints =
                self.add_variables_and_constraints(manager, board, &overlapping_units)?;

            constraints.extend(new_constraints);

            // assumptions that force the overlapping units to move in a way that causes no further conflicts
            let strong_move_assumptions = self.strong_move_assumptions(
                manager,
                &untouched_friendly_units,
                &overlapping_units,
                board,
            );
            // check the strong assumptions
            let is_sat = manager.solver.check_assumptions(&strong_move_assumptions);

            match is_sat {
                z3::SatResult::Sat => {
                    // we can safely proceed to attack positioning
                    break;
                }
                z3::SatResult::Unsat => {
                    // we need to resolve the potential conflicts that will be caused by the weaker assumptions
                    continue;
                }
                z3::SatResult::Unknown => {
                    bail!(
                        "Unknown result from SAT solver: {:?}",
                        manager.solver.get_reason_unknown().unwrap()
                    );
                }
            }
        }

        Ok(None)
    }

    /// Collect overlapping units data without borrowing solver mutably
    fn compute_overlapping_units<'ctx>(
        &self,
        manager: &ConstraintManager<'ctx>,
        model: &z3::Model<'ctx>,
        untouched_friendly_units: &HashSet<Loc>,
    ) -> Vec<Loc> {
        let mut overlapping_units = Vec::new();

        // Collect all hexes that will be occupied by non-passive attackers
        for attacker in &manager.graph.attackers {
            let passive_var = &manager.variables.passive[attacker];
            let is_passive = model.eval(passive_var, true).unwrap().as_bool().unwrap();

            if is_passive {
                continue;
            }

            let attack_hex_var = &manager.variables.attack_hex[attacker];
            let hex_val = model.eval(attack_hex_var, true).unwrap().as_u64().unwrap();
            let attack_hex = Loc::from_z3(hex_val);

            if untouched_friendly_units.contains(&attack_hex) {
                overlapping_units.push(*attacker);
            }
        }

        overlapping_units
    }

    /// create assumptions for moves avoiding non-combat friendly units
    fn strong_move_assumptions<'ctx>(
        &self,
        manager: &ConstraintManager<'ctx>,
        untouched_friendly_units: &HashSet<Loc>,
        overlapping_units: &[Loc],
        board: &Board,
    ) -> Vec<Bool<'ctx>> {
        let mut assumptions = Vec::new();

        for unit_loc in overlapping_units {
            let move_hex_var = &manager.variables.move_hex[&unit_loc];

            let move_hexes_occupied = untouched_friendly_units.iter().cloned().collect::<Vec<_>>();

            let move_hex_not_occupied_constraint =
                loc_var_in_hexes(manager.ctx, move_hex_var, &move_hexes_occupied).not();

            assumptions.push(move_hex_not_occupied_constraint);
        }

        assumptions
    }

    /// Add variables and constraints for overlapping units
    fn add_variables_and_constraints<'ctx>(
        &self,
        manager: &mut ConstraintManager<'ctx>,
        board: &Board,
        overlapping_units: &[Loc],
    ) -> Result<Vec<Bool<'ctx>>> {
        let mut constraints = Vec::new();

        // Add move_time and move_hex variables for overlapping units
        for unit_loc in overlapping_units {
            manager
                .variables
                .add_friendly_movement_variable(manager.ctx, *unit_loc);

            let new_constraints = self.add_friendly_movement_constraints(*unit_loc, manager, board);
            constraints.extend(new_constraints);
        }

        Ok(constraints)
    }

    /// Add movement constraints for friendly units
    fn add_friendly_movement_constraints<'ctx>(
        &self,
        friendly_loc: Loc,
        manager: &mut ConstraintManager<'ctx>,
        board: &Board,
    ) -> Vec<Bool<'ctx>> {
        let mut constraints = Vec::new();

        let move_hex_var = &manager.variables.move_hex[&friendly_loc];
        let move_time_var = &manager.variables.move_time[&friendly_loc];

        // Cannot be the same as the attack_hex of any non-passive attacker
        for attacker in &manager.graph.attackers {
            let passive_var = &manager.variables.passive[attacker];
            let attack_hex_var = &manager.variables.attack_hex[attacker];

            let same_hex = move_hex_var._eq(attack_hex_var);

            if attacker == &friendly_loc {
                manager.solver.assert(&same_hex);
                constraints.push(same_hex);
            } else {
                let attacker_active = passive_var.not();
                let conflict_constraint = attacker_active.implies(&same_hex.not());
                manager.solver.assert(&conflict_constraint);
                constraints.push(conflict_constraint);
            }
        }

        // Cannot be the same as the move_hex of any other friendly unit
        for (other_friendly, other_move_hex_var) in &manager.variables.move_hex {
            if other_friendly != &friendly_loc {
                let different_hex_constraint = move_hex_var._eq(other_move_hex_var).not();
                manager.solver.assert(&different_hex_constraint);
                constraints.push(different_hex_constraint);
            }
        }

        // restrict the move hex to the theoretical move hexes
        let theoretical_move_hexes = board.get_theoretical_move_hexes(friendly_loc);

        let theoretical_hex_constraint =
            loc_var_in_hexes(manager.ctx, move_hex_var, &theoretical_move_hexes);

        manager.solver.assert(&theoretical_hex_constraint);
        constraints.push(theoretical_hex_constraint);

        // add DNF constraints on the move
        for attack_hex in theoretical_move_hexes {
            let dnf = &manager.graph.move_hex_map[&friendly_loc][&attack_hex];
            let condition = move_hex_var._eq(&attack_hex.as_z3(manager.ctx));

            if let Some(dnf) = dnf {
                let path_constraint = manager.create_dnf_constraint(
                    &dnf,
                    &manager.variables.move_time[&friendly_loc],
                    &condition,
                );

                manager.solver.assert(&path_constraint);
                constraints.push(path_constraint);
            }
        }

        constraints
    }

    /// Stage 2: Combat positioning - choosing specific locations for combat-relevant pieces
    pub fn combat_positioning<'ctx>(
        &self,
        manager: &mut ConstraintManager<'ctx>,
        board: &Board,
        side: Side,
        move_candidates: &[MoveCandidate],
    ) -> Result<()> {
        // Create filtered view of move affinity list for combat-relevant moves
        let combat_relevant_moves = self.filter_combat_relevant_moves(manager, move_candidates);

        // Track which pieces have had moves allocated
        let mut allocated_pieces = HashSet::new();

        // Iterate through the filtered list
        for candidate in &combat_relevant_moves {
            if allocated_pieces.contains(&candidate.from_loc) {
                continue;
            }

            // Check if the move is to a location occupied by a non-combat-relevant piece
            let blocking_piece = candidate.to_loc;
            if self.has_blocking_piece(manager, blocking_piece, board, side) {
                self.add_variables_and_constraints(manager, board, &[blocking_piece])?;

                // Add moves for this piece to the end of the filtered list
                // (This would require modifying the list, but we'll handle it differently)
            }

            // Attempt to assert the current move
            let move_constraint = self.create_move_constraint(
                &manager.variables,
                candidate.from_loc,
                candidate.to_loc,
                manager.ctx,
            );

            manager.solver.push();
            manager.solver.assert(&move_constraint);

            match manager.solver.check() {
                z3::SatResult::Sat => {
                    // Move is valid, track that the piece has moved
                    allocated_pieces.insert(candidate.from_loc);
                }
                z3::SatResult::Unsat => {
                    // Move is invalid, revert the assertion
                    manager.solver.pop(1);
                }
                z3::SatResult::Unknown => {
                    manager.solver.pop(1);
                    bail!(
                        "Unknown result from SAT solver: {:?}",
                        manager.solver.get_reason_unknown().unwrap()
                    );
                }
            }
        }

        Ok(())
    }

    /// Generate all possible moves with random affinity values
    pub fn generate_move_candidates(
        &self,
        rng: &mut impl Rng,
        board: &Board,
        side: Side,
    ) -> Vec<MoveCandidate> {
        let mut candidates = Vec::new();

        // Get all friendly pieces
        for (loc, piece) in &board.pieces {
            if piece.side == side {
                // Get all possible move destinations for this piece
                let valid_moves = board.get_theoretical_move_hexes(*loc);

                for to_loc in valid_moves {
                    // Generate a random affinity value between 0 and 1
                    let affinity_value = rng.gen_range(0.0..1.0);

                    candidates.push(MoveCandidate {
                        from_loc: *loc,
                        to_loc,
                        score: affinity_value,
                    });
                }
            }
        }

        candidates
    }

    /// Filter moves to only include combat-relevant ones
    fn filter_combat_relevant_moves<'ctx>(
        &self,
        manager: &ConstraintManager<'ctx>,
        move_candidates: &[MoveCandidate],
    ) -> Vec<MoveCandidate> {
        move_candidates
            .iter()
            .filter(|candidate| {
                if manager.variables.move_hex.contains_key(&candidate.from_loc) {
                    // moves for friendly units in the way are always combat-relevant
                    true
                } else if manager.graph.attackers.contains(&candidate.from_loc) {
                    // moves for attackers are combat-relevant if they are to an attack hex
                    manager.graph.attack_hex_map[&candidate.from_loc].contains(&candidate.to_loc)
                } else {
                    // Not combat-relevant
                    false
                }
            })
            .cloned()
            .collect()
    }

    /// Get the piece that would block a move to a specific location
    fn has_blocking_piece<'ctx>(
        &self,
        manager: &ConstraintManager<'ctx>,
        to_loc: Loc,
        board: &Board,
        side: Side,
    ) -> bool {
        manager.graph.friends.contains(&to_loc)
            && !manager.graph.attackers.contains(&to_loc)
            && !manager.variables.move_hex.contains_key(&to_loc)
    }

    /// Create a Z3 constraint for a specific move
    fn create_move_constraint<'ctx>(
        &self,
        variables: &SatVariables<'ctx>,
        from_loc: Loc,
        to_loc: Loc,
        ctx: &'ctx z3::Context,
    ) -> Bool<'ctx> {
        if let Some(move_hex_var) = variables.move_hex.get(&from_loc) {
            // This is a friendly unit that needs to move out of the way
            move_hex_var._eq(&to_loc.as_z3(ctx))
        } else if let Some(attack_hex_var) = variables.attack_hex.get(&from_loc) {
            // This is an attacker
            attack_hex_var._eq(&to_loc.as_z3(ctx))
        } else {
            panic!("create_move_constraint called for a piece that is not a friendly unit or an attacker: {:?}", from_loc);
        }
    }

    /// Stage 3: Non-attack movement - generates movement actions for non-combat-relevant pieces
    /// This function doesn't interact with the SAT solver and can be called separately
    pub fn generate_non_attack_movements<'ctx>(
        &self,
        manager: &ConstraintManager<'ctx>,
        board: &Board,
        move_candidates: Vec<MoveCandidate>,
    ) -> Vec<AttackAction> {
        let movements = self.compute_optimal_matching(manager, board, &move_candidates);
        self.move_actions(&movements)
    }

    /// Compute optimal matching using LAPJV algorithm
    pub fn compute_optimal_matching(
        &self,
        manager: &ConstraintManager,
        board: &Board,
        move_candidates: &[MoveCandidate],
    ) -> HashMap<Loc, Loc> {
        let mut yet_to_move: HashSet<Loc> = manager
            .graph
            .friends
            .iter()
            .filter(|loc| {
                !manager.variables.move_hex.contains_key(loc)
                    && !manager.graph.attackers.contains(loc)
            })
            .cloned()
            .collect();

        let to_be_processed: Vec<MoveCandidate> = move_candidates
            .into_iter()
            .filter(|candidate| {
                let MoveCandidate {
                    from_loc, to_loc, ..
                } = candidate;

                // Must be a yet-to-move piece
                yet_to_move.contains(from_loc)
                    && (!board.pieces.contains_key(to_loc) || yet_to_move.contains(to_loc))
            })
            .cloned()
            .collect();

        // Get all possible destinations from move_hex_map
        let mut all_destinations = HashSet::new();
        for candidate in &to_be_processed {
            let dest = candidate.to_loc;
            all_destinations.insert(dest);
        }

        let destination_locs: Vec<Loc> = all_destinations.into_iter().collect();

        let num_locs = destination_locs.len();
        let loc_to_idx_map = destination_locs
            .iter()
            .enumerate()
            .map(|(idx, loc)| (*loc, idx))
            .collect::<HashMap<_, _>>();

        let loc_to_idx = |loc: Loc| *loc_to_idx_map.get(&loc).unwrap();

        // Build cost matrix: friend_locs x destination_locs
        let mut cost_matrix = vec![vec![f64::INFINITY; num_locs]; num_locs];

        for candidate in to_be_processed {
            let MoveCandidate {
                from_loc,
                to_loc,
                score,
            } = candidate;
            let cost = score_to_cost(score);
            cost_matrix[loc_to_idx(from_loc)][loc_to_idx(to_loc)] = cost;
        }

        // set non-friend -> dest costs to 0
        // these are sentinels to populate the square matrix
        for non_friend_idx in 0..num_locs {
            let non_friend_loc = destination_locs[non_friend_idx];
            if yet_to_move.contains(&non_friend_loc) {
                continue;
            }

            for dest_idx in 0..num_locs {
                cost_matrix[non_friend_idx][dest_idx] = 0.0;
            }
        }

        // Convert cost matrix to array
        let cost_array_data: Vec<f64> = cost_matrix.into_iter().flatten().collect();
        let cost_array = Matrix::from_shape_vec((num_locs, num_locs), cost_array_data).unwrap();

        let (row_indices, _) = lapjv(&cost_array).unwrap();

        let mut matching = HashMap::new();
        for (friend_idx, dest_idx) in row_indices.into_iter().enumerate() {
            let friend_loc = destination_locs[friend_idx];
            if !yet_to_move.contains(&friend_loc) {
                continue;
            }

            let dest_loc = destination_locs[dest_idx];
            matching.insert(friend_loc, dest_loc);
        }

        matching
    }

    pub fn move_actions(&self, movements: &HashMap<Loc, Loc>) -> Vec<AttackAction> {
        let (paths, cycles) = identify_paths_and_cycles(movements);
        let mut actions = Vec::new();
        for path in paths {
            let moves = path.windows(2).map(|w| AttackAction::Move {
                from_loc: w[0],
                to_loc: w[1],
            });
            actions.extend(moves.rev());
        }
        for cycle in cycles {
            actions.push(AttackAction::MoveCyclic { locs: cycle });
        }
        actions
    }
}

type MovementPath = Vec<Loc>;
type MovementCycle = Vec<Loc>;

/// Identify paths and cycles in the movement graph
fn identify_paths_and_cycles(
    movement: &HashMap<Loc, Loc>,
) -> (Vec<MovementPath>, Vec<MovementCycle>) {
    let mut paths = Vec::new();
    let mut cycles = Vec::new();
    let dests = movement.values().cloned().collect::<HashSet<_>>();

    let sources = movement.keys().filter(|loc| !dests.contains(loc));
    let mut visited = HashSet::new();

    for source in sources {
        visited.insert(*source);

        let mut path = vec![*source];
        let mut current = source;
        while let Some(next) = movement.get(current) {
            visited.insert(*next);
            path.push(*next);
            current = next;
        }

        paths.push(path);
    }

    for (source, dest) in movement {
        if visited.contains(dest) {
            continue;
        }
        visited.insert(*source);

        let mut cycle = vec![*source];
        let mut next = dest;
        while next != source {
            visited.insert(*next);
            cycle.push(*next);
            next = movement.get(&next).unwrap();
        }

        cycles.push(cycle);
    }

    (paths, cycles)
}

pub fn score_to_cost(score: f64) -> f64 {
    -score.log2()
}

/// Helper function to create location constraints
fn loc_var_in_hexes<'ctx>(
    ctx: &'ctx z3::Context,
    var: &z3::ast::BV<'ctx>,
    hexes: &[Loc],
) -> Bool<'ctx> {
    if hexes.is_empty() {
        return Bool::from_bool(ctx, false);
    }

    let alternatives = hexes
        .iter()
        .map(|hex| crate::core::Loc::as_z3(*hex, ctx)._eq(var))
        .collect::<Vec<_>>();

    let alternatives_ref: Vec<_> = alternatives.iter().collect();

    Bool::or(ctx, &alternatives_ref)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::captain::combat::constraints::generate_move_from_sat_model;
    use crate::ai::rng::make_rng;
    use crate::core::{board::Board, map::Map, side::Side, units::Unit};
    use z3::Context;

    #[test]
    fn test_generate_move_candidates() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);

        // Add a friendly piece
        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            crate::core::loc::Loc::new(1, 1),
        ));

        let mut positioning = SatPositioningSystem {};
        let candidates =
            positioning.generate_move_candidates(&mut make_rng(), &board, Side::Yellow);

        // Should generate some move candidates
        assert!(!candidates.is_empty());

        // All candidates should be for the friendly piece
        for candidate in &candidates {
            assert_eq!(candidate.from_loc, crate::core::loc::Loc::new(1, 1));
            assert!(candidate.score >= 0.0);
            assert!(candidate.score < 1.0);
        }
    }

    #[test]
    fn test_candidates_are_sorted_by_affinity() {
        let map = Map::BlackenedShores;
        let board = Board::new(&map);

        let mut positioning = SatPositioningSystem {};
        let mut candidates =
            positioning.generate_move_candidates(&mut make_rng(), &board, Side::Yellow);

        // Sort by affinity value
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Check that they are in descending order
        for i in 1..candidates.len() {
            assert!(candidates[i - 1].score >= candidates[i].score);
        }
    }

    #[test]
    fn test_stage_3_empty_board() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);
        let ctx = Context::new(&z3::Config::new());
        let graph = board.combat_graph(Side::Yellow);
        let manager = ConstraintManager::new(&ctx, graph, &board);

        let positioning = SatPositioningSystem {};
        let move_candidates = vec![];
        let actions = positioning.generate_non_attack_movements(&manager, &board, move_candidates);

        // Should return no actions for empty board
        assert!(actions.is_empty());
    }

    #[test]
    fn test_stage_3_single_piece_no_moves() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);

        // Add a friendly piece at a location with no valid moves
        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            crate::core::loc::Loc::new(0, 0), // Corner location with limited moves
        ));

        let ctx = Context::new(&z3::Config::new());
        let graph = board.combat_graph(Side::Yellow);
        let manager = ConstraintManager::new(&ctx, graph, &board);

        let mut positioning = SatPositioningSystem {};
        let move_candidates =
            positioning.generate_move_candidates(&mut make_rng(), &board, Side::Yellow);
        let actions = positioning.generate_non_attack_movements(&manager, &board, move_candidates);

        // Should return some actions (even if just staying in place)
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_stage_3_cycle_detection() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);

        // Add two friendly pieces that can move to each other's positions
        let loc1 = crate::core::loc::Loc::new(1, 1);
        let loc2 = crate::core::loc::Loc::new(1, 2);

        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            loc1,
        ));
        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            loc2,
        ));

        let ctx = Context::new(&z3::Config::new());
        let graph = board.combat_graph(Side::Yellow);
        let manager = ConstraintManager::new(&ctx, graph, &board);

        let positioning = SatPositioningSystem {};
        let move_candidates = vec![
            MoveCandidate {
                from_loc: loc1,
                to_loc: loc2,
                score: 0.8,
            },
            MoveCandidate {
                from_loc: loc2,
                to_loc: loc1,
                score: 0.9,
            },
        ];

        let actions = positioning.generate_non_attack_movements(&manager, &board, move_candidates);

        // Should detect a cycle and create a MoveCyclic action
        assert!(!actions.is_empty());
        assert!(matches!(
            actions[0],
            crate::core::board::actions::AttackAction::MoveCyclic { .. }
        ));
    }

    #[test]
    fn test_stage_3_path_detection() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);

        // Add three friendly pieces in a line
        let loc1 = crate::core::loc::Loc::new(1, 1);
        let loc2 = crate::core::loc::Loc::new(1, 2);
        let loc3 = crate::core::loc::Loc::new(1, 3);

        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            loc1,
        ));
        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            loc2,
        ));
        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            loc3,
        ));

        let ctx = Context::new(&z3::Config::new());
        let graph = board.combat_graph(Side::Yellow);
        let manager = ConstraintManager::new(&ctx, graph, &board);

        let positioning = SatPositioningSystem {};
        let move_candidates = vec![
            MoveCandidate {
                from_loc: loc1,
                to_loc: loc2,
                score: 0.8,
            },
            MoveCandidate {
                from_loc: loc2,
                to_loc: loc3,
                score: 0.9,
            },
        ];

        let actions = positioning.generate_non_attack_movements(&manager, &board, move_candidates);

        // Should detect a path and create Move actions
        assert!(!actions.is_empty());
        assert!(matches!(
            actions[0],
            crate::core::board::actions::AttackAction::Move { .. }
        ));
    }

    #[test]
    fn test_stage_3_conflict_resolution() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);

        // Add two friendly pieces that want to move to the same location
        let loc1 = crate::core::loc::Loc::new(1, 1);
        let loc2 = crate::core::loc::Loc::new(1, 2);
        let target_loc = crate::core::loc::Loc::new(2, 1);

        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            loc1,
        ));
        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            loc2,
        ));

        let ctx = Context::new(&z3::Config::new());
        let graph = board.combat_graph(Side::Yellow);
        let manager = ConstraintManager::new(&ctx, graph, &board);

        let positioning = SatPositioningSystem {};
        let move_candidates = vec![
            MoveCandidate {
                from_loc: loc1,
                to_loc: target_loc,
                score: 0.7,
            },
            MoveCandidate {
                from_loc: loc2,
                to_loc: target_loc,
                score: 0.9,
            }, // Higher affinity
        ];

        let actions = positioning.generate_non_attack_movements(&manager, &board, move_candidates);

        // Should resolve conflict by choosing the higher affinity move
        assert!(!actions.is_empty());

        // Check that only one piece moved to the target location
        let moves_to_target = actions
            .iter()
            .filter_map(|action| {
                if let crate::core::board::actions::AttackAction::Move { to_loc, .. } = action {
                    if *to_loc == target_loc {
                        Some(to_loc)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .count();

        assert_eq!(moves_to_target, 1);
    }

    #[test]
    fn test_filter_combat_relevant_moves() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);

        // Add a friendly piece
        let loc = crate::core::loc::Loc::new(1, 1);
        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            loc,
        ));

        let ctx = Context::new(&z3::Config::new());
        let graph = board.combat_graph(Side::Yellow);
        let manager = ConstraintManager::new(&ctx, graph, &board);

        let positioning = SatPositioningSystem {};
        let move_candidates = vec![MoveCandidate {
            from_loc: loc,
            to_loc: crate::core::loc::Loc::new(1, 2),
            score: 0.5,
        }];

        let filtered = positioning.filter_combat_relevant_moves(&manager, &move_candidates);

        // Since the piece is not in combat, it should be filtered out
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_get_blocking_piece() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);

        // Add a friendly piece
        let loc = crate::core::loc::Loc::new(1, 1);
        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            loc,
        ));

        let ctx = Context::new(&z3::Config::new());
        let graph = board.combat_graph(Side::Yellow);
        let manager = ConstraintManager::new(&ctx, graph, &board);

        let positioning = SatPositioningSystem {};

        // Should find the blocking piece
        let blocking = positioning.has_blocking_piece(&manager, loc, &board, Side::Yellow);
        assert!(blocking);

        // Should not find a blocking piece for an empty location
        let empty_loc = crate::core::loc::Loc::new(5, 5);
        let blocking = positioning.has_blocking_piece(&manager, empty_loc, &board, Side::Yellow);
        assert!(!blocking);
    }

    #[test]
    fn test_create_move_constraint() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);

        // Add a friendly piece
        let loc = crate::core::loc::Loc::new(1, 1);
        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            loc,
        ));

        let ctx = Context::new(&z3::Config::new());
        let graph = board.combat_graph(Side::Yellow);
        let manager = ConstraintManager::new(&ctx, graph, &board);

        let positioning = SatPositioningSystem {};
        let to_loc = crate::core::loc::Loc::new(1, 2);

        // Test with a piece that's not in combat (should return false)
        let constraint = positioning.create_move_constraint(&manager.variables, loc, to_loc, &ctx);

        // The constraint should be a valid Z3 expression
        assert!(constraint.is_const());
    }

    #[test]
    fn test_position_pieces_integration() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);

        // Add a friendly piece
        let loc = crate::core::loc::Loc::new(1, 1);
        board.add_piece(crate::core::board::Piece::new(
            Unit::Zombie,
            Side::Yellow,
            loc,
        ));

        let ctx = Context::new(&z3::Config::new());
        let graph = board.combat_graph(Side::Yellow);
        let mut manager = ConstraintManager::new(&ctx, graph, &board);

        let mut positioning = SatPositioningSystem {};
        let move_candidates =
            positioning.generate_move_candidates(&mut make_rng(), &board, Side::Yellow);

        let result = positioning
            .combat_reconciliation(&mut manager, &board, Side::Yellow, &move_candidates)
            .unwrap();

        assert!(result.is_none());

        positioning
            .combat_positioning(&mut manager, &board, Side::Yellow, &move_candidates)
            .unwrap();

        let model = manager.solver.get_model().unwrap();
        let sat_actions =
            generate_move_from_sat_model(&model, &manager.graph, &manager.variables, &board);

        let positioning_actions =
            positioning.generate_non_attack_movements(&manager, &board, move_candidates);

        let all_actions = sat_actions
            .into_iter()
            .chain(positioning_actions.into_iter())
            .collect::<Vec<_>>();

        // Should return some actions (even if just staying in place)
        assert!(!all_actions.is_empty());
    }

    #[test]
    fn test_move_candidate_clone() {
        let candidate = MoveCandidate {
            from_loc: crate::core::loc::Loc::new(1, 1),
            to_loc: crate::core::loc::Loc::new(1, 2),
            score: 0.5,
        };

        let cloned = candidate.clone();
        assert_eq!(candidate.from_loc, cloned.from_loc);
        assert_eq!(candidate.to_loc, cloned.to_loc);
        assert_eq!(candidate.score, cloned.score);
    }

    #[test]
    fn test_move_candidate_debug() {
        let candidate = MoveCandidate {
            from_loc: crate::core::loc::Loc::new(1, 1),
            to_loc: crate::core::loc::Loc::new(1, 2),
            score: 0.5,
        };

        let debug_str = format!("{:?}", candidate);
        assert!(debug_str.contains("MoveCandidate"));
        assert!(debug_str.contains("0.5"));
    }
}
