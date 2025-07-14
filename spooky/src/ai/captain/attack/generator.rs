use anyhow::{bail, Context as AnyhowContext, Result};
use core::num;
use lapjv::{lapjv, Matrix};
use rand::{rngs::StdRng, *};
use std::collections::{HashMap, HashSet};
use z3::{
    ast::{Ast, Bool, BV},
    Context, Model, SatResult, Solver,
};

use crate::{
    ai::{
        captain::attack::{
            constraints::{generate_move_from_model, ConstraintManager},
            graph::RemovalDNF,
            prophet::{AttackAssumption, MoveAssumption},
        },
        rng,
    },
    core::{
        board::{actions::AttackAction, Board},
        loc::{Loc, Move},
        side::Side,
        units::{Attack, Unit},
    },
};

use super::{
    constraints::SatVariables,
    graph::CombatGraph,
    prophet::{AttackConstraint, DeathProphet},
};

const BV_TIME_SIZE: u32 = 4;
type Z3Time<'ctx> = BV<'ctx>;

const BV_LOC_SIZE: u32 = 8;
type Z3Loc<'ctx> = BV<'ctx>;

pub type MovementPath = Vec<Loc>;
pub type MovementCycle = Vec<Loc>;

#[derive(Clone, Debug)]
pub enum Assumption {
    Move(MoveAssumption),
    Attack(AttackAssumption),
}

impl Assumption {
    pub fn cost(&self) -> f64 {
        match self {
            Assumption::Move(move_assumption) => move_assumption.cost,
            Assumption::Attack(attack_assumption) => attack_assumption.cost,
        }
    }
}

pub struct Z3Assumption<'ctx> {
    pub constraint: Bool<'ctx>,
    pub assumption: Assumption,
}

/// New attack generator that uses death prophet priorities and optimal matching
pub struct AttackGenerator<'ctx> {
    manager: ConstraintManager<'ctx>,
    death_prophet: DeathProphet,
    // Precomputed DNFs for each move
    // move_dnfs: HashMap<Move, Vec<Vec<Loc>>>,
}

fn get_move_dnfs(graph: &CombatGraph) -> HashMap<Move, Vec<Vec<Loc>>> {
    let mut move_dnfs = HashMap::new();
    for (friend, hex_map) in &graph.move_hex_map {
        for (dest, dnf) in hex_map {
            move_dnfs.insert(
                Move {
                    from: *friend,
                    to: *dest,
                },
                dnf.clone(),
            );
        }
    }
    move_dnfs
}

impl<'ctx> AttackGenerator<'ctx> {
    pub fn new(ctx: &'ctx Context, board: &Board, side: Side, rng: &mut impl Rng) -> Self {
        let death_prophet = DeathProphet::new(StdRng::from_rng(rng).unwrap());
        let graph = board.combat_graph(side);

        let manager = ConstraintManager::new(ctx, graph, board);

        Self {
            manager,
            death_prophet,
        }
    }

    /// Generate attack actions using the new strategy
    pub fn generate_attacks(&mut self, board: &Board, side: Side) -> Result<Vec<AttackAction>> {
        // Step 1: Generate priorities for all assumptions
        let prophecy = self.death_prophet.make_prophecy(board, side);
        let mut cant_move = HashSet::new();
        let mut unassumable = HashSet::new();

        let base_move_dnfs = get_move_dnfs(&self.manager.graph);

        // Step 2: Iteratively solve
        loop {
            // Filter priorities to exclude unassumable removes/keeps and cant_move moves
            let move_assumptions: Vec<_> = prophecy
                .move_assumptions
                .iter()
                .filter(|assumption| !cant_move.contains(&assumption.mv))
                .cloned()
                .collect();

            let mut attack_assumptions = Vec::new();
            let mut locs_with_assumptions = HashSet::new();
            let mut removals = HashSet::new();

            for assumption in prophecy.attack_assumptions.iter() {
                let constraint = &assumption.constraint;
                let loc = constraint.loc();

                if locs_with_assumptions.contains(&loc) {
                    continue;
                }

                if unassumable.contains(constraint) {
                    continue;
                }

                attack_assumptions.push(assumption.clone());
                locs_with_assumptions.insert(loc);
                if let AttackConstraint::Remove(_) = constraint {
                    removals.insert(loc);
                }
            }

            let dnf_map = base_move_dnfs
                .iter()
                .map(|(mv, dnf)| {
                    let new_dnf = dnf
                        .iter()
                        .filter(|locs| locs.iter().all(|loc| !removals.contains(loc)))
                        .cloned()
                        .collect();

                    (mv.clone(), new_dnf)
                })
                .collect::<HashMap<_, _>>();

            // 2a: Compute optimal matching using LAPJV
            let movement = match self.compute_optimal_matching(&move_assumptions, &dnf_map) {
                Some(matching) => matching,
                None => {
                    let attack_assumption_to_remove = attack_assumptions.pop().unwrap();
                    unassumable.insert(attack_assumption_to_remove.constraint);
                    cant_move.clear();
                    continue;
                }
            };

            // 2b: Identify paths and cycles in the movement graph
            let (paths, cycles) = identify_paths_and_cycles(&movement);

            // 2c: Compute move time constraints and build SAT assumptions
            let z3assumptions = self.compute_z3_assumptions(
                &paths,
                &cycles,
                &move_assumptions,
                &attack_assumptions,
                &dnf_map,
            );

            let checkable_assumptions: Vec<_> = z3assumptions
                .iter()
                .map(|assumption| assumption.constraint.clone())
                .collect();

            // 2d: Query SAT with assumptions
            let result = self
                .manager
                .solver
                .check_assumptions(&checkable_assumptions);

            match result {
                SatResult::Sat => {
                    let model = self
                        .manager
                        .solver
                        .get_model()
                        .context("Failed to get model")?;

                    println!("Model: {:#?}", model);

                    let actions = generate_move_from_model(
                        &model,
                        &movement,
                        &self.manager.graph,
                        &self.manager.variables,
                        board,
                    );

                    println!(
                        "Actions:\n{}",
                        actions
                            .iter()
                            .map(|action| action.to_string())
                            .collect::<Vec<_>>()
                            .join("\n")
                    );

                    println!("DNF: {:#?}", dnf_map);

                    return Ok(actions);
                }
                SatResult::Unsat => {
                    let unsat_core = self.manager.solver.get_unsat_core();
                    let worst_assumption = self
                        .get_worst_assumption(&unsat_core, &z3assumptions)
                        .context("Failed to get worst assumption")?;

                    match worst_assumption {
                        Assumption::Move(move_assumption) => {
                            cant_move.insert(move_assumption.mv);
                        }
                        Assumption::Attack(attack_assumption) => {
                            unassumable.insert(attack_assumption.constraint);
                        }
                    }
                }
                SatResult::Unknown => {
                    bail!(
                        "Unknown result from SAT solver: {:?}",
                        self.manager.solver.get_reason_unknown().unwrap()
                    );
                }
            }
        }
    }

    /// Compute optimal matching using LAPJV algorithm
    /// Returns None if no valid matching is found
    fn compute_optimal_matching(
        &mut self,
        move_assumptions: &Vec<MoveAssumption>,
        dnf_map: &HashMap<Move, Vec<Vec<Loc>>>,
    ) -> Option<HashMap<Loc, Loc>> {
        // Get all possible destinations from move_hex_map
        let mut all_destinations = HashSet::new();
        for (mv, dnf) in dnf_map {
            if dnf.is_empty() {
                continue;
            }
            let dest = mv.to;
            all_destinations.insert(dest);
        }

        let friend_locs: Vec<Loc> = self.manager.graph.friends.iter().cloned().collect();
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

        for assumption in move_assumptions {
            let MoveAssumption {
                mv: Move { from, to },
                cost,
            } = assumption;
            cost_matrix[loc_to_idx(*from)][loc_to_idx(*to)] = *cost;
        }

        // set non-friend -> dest costs to 0
        // these are sentinels to populate the square matrix
        for non_friend_idx in 0..num_locs {
            let non_friend_loc = destination_locs[non_friend_idx];
            if friend_locs.contains(&non_friend_loc) {
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
            if !friend_locs.contains(&friend_loc) {
                continue;
            }

            let dest_loc = destination_locs[dest_idx];

            if !self
                .manager
                .graph
                .move_hex_map
                .get(&friend_loc)
                .unwrap()
                .contains_key(&dest_loc)
            {
                return None;
            }

            matching.insert(friend_loc, dest_loc);
        }

        Some(matching)
    }

    /// Compute move time constraints for the movement graph
    fn compute_z3_assumptions(
        &self,
        paths: &[MovementPath],
        cycles: &[MovementCycle],
        move_assumptions: &[MoveAssumption],
        attack_assumptions: &[AttackAssumption],
        dnf_map: &HashMap<Move, RemovalDNF>,
    ) -> Vec<Z3Assumption<'ctx>> {
        let mut assumptions = Vec::new();
        let move_assumption_map = move_assumptions
            .iter()
            .map(|assumption| (assumption.mv, assumption))
            .collect::<HashMap<_, _>>();

        // Add constraints for paths
        for path in paths {
            let path_assumptions =
                self.compute_z3_path_assumptions(path, dnf_map, &move_assumption_map);
            assumptions.extend(path_assumptions);
        }

        // Add constraints for cycles
        for cycle in cycles {
            let cycle_assumptions =
                self.compute_z3_cycle_assumptions(cycle, dnf_map, &move_assumption_map);
            assumptions.extend(cycle_assumptions);
        }

        let z3_attack_assumptions = attack_assumptions
            .iter()
            .map(|assumption| {
                let constraint_bool = match assumption.constraint {
                    AttackConstraint::Remove(loc) => self.manager.variables.removed[&loc].clone(),
                    AttackConstraint::Keep(loc) => self.manager.variables.removed[&loc].not(),
                };
                Z3Assumption {
                    constraint: constraint_bool,
                    assumption: Assumption::Attack(assumption.clone()),
                }
            })
            .collect::<Vec<_>>();

        assumptions.extend(z3_attack_assumptions);

        assumptions
    }

    fn compute_z3_path_assumptions(
        &self,
        path: &MovementPath,
        dnf_map: &HashMap<Move, RemovalDNF>,
        move_assumption_map: &HashMap<Move, &MoveAssumption>,
    ) -> Vec<Z3Assumption<'ctx>> {
        let mut assumptions = Vec::new();

        // track relevant properties for the segment
        let mut dnfs = Vec::new();
        let mut last_attacker = None;

        let moves = path_to_moves(path);

        for (i, mv) in moves.iter().enumerate().rev() {
            let mut assumption = *move_assumption_map.get(mv).unwrap();

            if let Some(dnf) = dnf_map.get(mv) {
                dnfs.push(dnf);
                let untimed_dnf_constraint = self.manager.create_untimed_dnf_constraint(dnf);
                assumptions.push(Z3Assumption {
                    constraint: untimed_dnf_constraint,
                    assumption: Assumption::Move(assumption.clone()),
                });
            }

            let attacker = mv.from;

            if !self.manager.graph.attackers.contains(&attacker) {
                continue;
            }

            let hex_var = &self.manager.variables.attack_hex[&attacker];
            let time_var = &self.manager.variables.attack_time[&attacker];

            assumptions.push(Z3Assumption {
                constraint: hex_var._eq(&mv.to.as_z3(self.manager.ctx)),
                assumption: Assumption::Move(assumption.clone()),
            });

            for (di, dnf) in dnfs.iter().rev().enumerate() {
                let dnf_constraint = self.manager.create_dnf_constraint(dnf, time_var);

                let dnf_assumption = *move_assumption_map.get(&moves[i + di]).unwrap();
                if dnf_assumption.cost > assumption.cost {
                    assumption = dnf_assumption;
                }

                assumptions.push(Z3Assumption {
                    constraint: dnf_constraint,
                    assumption: Assumption::Move(assumption.clone()),
                });
            }

            if let Some(last_attacker) = last_attacker {
                let last_time_var = &self.manager.variables.attack_time[&last_attacker];
                assumptions.push(Z3Assumption {
                    constraint: time_var.bvuge(last_time_var),
                    assumption: Assumption::Move(assumption.clone()),
                });
            }

            // reset segment tracking
            dnfs.clear();
            last_attacker = Some(attacker);
        }

        assumptions
    }

    fn compute_z3_cycle_assumptions(
        &self,
        cycle: &MovementCycle,
        dnf_map: &HashMap<Move, RemovalDNF>,
        move_assumption_map: &HashMap<Move, &MoveAssumption>,
    ) -> Vec<Z3Assumption<'ctx>> {
        let mut assumptions = Vec::new();

        let attacker = cycle
            .iter()
            .find(|loc| self.manager.graph.attackers.contains(loc));
        let time_var = match attacker {
            Some(attacker) => &self.manager.variables.attack_time[&attacker],
            None => return assumptions,
        };

        let moves = cycle_to_moves(cycle);

        let worst_assumption = *moves
            .iter()
            .map(|mv| move_assumption_map.get(mv).unwrap())
            .max_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap())
            .unwrap();

        for mv in moves {
            let cur_assumption = *move_assumption_map.get(&mv).unwrap();

            if let Some(dnf) = dnf_map.get(&mv) {
                let untimed_dnf_constraint = self.manager.create_untimed_dnf_constraint(dnf);
                assumptions.push(Z3Assumption {
                    constraint: untimed_dnf_constraint,
                    assumption: Assumption::Move(cur_assumption.clone()),
                });

                let dnf_constraint = self.manager.create_dnf_constraint(dnf, time_var);
                assumptions.push(Z3Assumption {
                    constraint: dnf_constraint,
                    assumption: Assumption::Move(worst_assumption.clone()),
                });
            }

            let attacker = mv.from;

            if !self.manager.graph.attackers.contains(&attacker) {
                continue;
            }

            let hex_var = &self.manager.variables.attack_hex[&attacker];
            let mv_time_var = &self.manager.variables.attack_time[&attacker];

            assumptions.push(Z3Assumption {
                constraint: hex_var._eq(&mv.to.as_z3(self.manager.ctx)),
                assumption: Assumption::Move(cur_assumption.clone()),
            });

            assumptions.push(Z3Assumption {
                constraint: mv_time_var._eq(time_var),
                assumption: Assumption::Move(worst_assumption.clone()),
            });
        }

        assumptions
    }

    /// Get the worst assumption from the unsat core
    fn get_worst_assumption(
        &self,
        unsat_core: &[Bool<'ctx>],
        assumptions: &[Z3Assumption<'ctx>],
    ) -> Option<Assumption> {
        let mut worst_assumption: Option<Assumption> = None;

        for constraint in unsat_core {
            let assumption = &assumptions
                .iter()
                .find(|assumption| &assumption.constraint == constraint)
                .unwrap()
                .assumption;

            if is_worse(&assumption, &worst_assumption) {
                worst_assumption = Some(assumption.clone());
            }
        }

        worst_assumption
    }
}

fn is_worse(a: &Assumption, b: &Option<Assumption>) -> bool {
    match b {
        Some(b) => a.cost() > b.cost(),
        None => true,
    }
}

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

fn path_to_moves(path: &MovementPath) -> Vec<Move> {
    path.windows(2)
        .map(|window| Move {
            from: window[0],
            to: window[1],
        })
        .collect()
}

fn cycle_to_moves(cycle: &MovementCycle) -> Vec<Move> {
    cycle
        .windows(2)
        .map(|window| Move {
            from: window[0],
            to: window[1],
        })
        .chain(std::iter::once(Move {
            from: *cycle.last().unwrap(),
            to: *cycle.first().unwrap(),
        }))
        .collect()
}
