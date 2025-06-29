use std::collections::{HashMap, HashSet};

use crate::core::{
    board::actions::AttackAction,
    board::Board,
    loc::Loc,
    side::Side,
    units::{Attack, Unit},
};

use z3::{
    ast::{Ast, Bool, Int, BV},
    Context, Model, SatResult, Solver,
};

use super::combat::{CombatGraph, CombatTriple};

const BV_HP_SIZE: u32 = 10;
type Z3HP<'ctx> = BV<'ctx>;

const BV_TIME_SIZE: u32 = 4;
type Z3Time<'ctx> = BV<'ctx>;

const BV_LOC_SIZE: u32 = 8;
type Z3Loc<'ctx> = BV<'ctx>;

/// Variables for the new SAT-based constraint satisfaction problem
pub struct SatVariables<'ctx> {
    // Movement variables
    pub move_hex: HashMap<Loc, Z3Loc<'ctx>>, // where the unit moves to
    pub move_time: HashMap<Loc, Z3Time<'ctx>>, // when the unit moves

    // Combat variables
    pub attacked: HashMap<(Loc, Loc), Bool<'ctx>>, // for each (attacker, defender), whether attacker attacks defender
    pub num_attacks: HashMap<(Loc, Loc), Z3HP<'ctx>>, // for each (attacker, defender), number of attacks from attacker to defender

    // Removal variables
    pub removed: HashMap<Loc, Bool<'ctx>>, // whether a unit is removed
    pub removal_time: HashMap<Loc, Z3Time<'ctx>>, // when the unit is removed
}

impl<'ctx> SatVariables<'ctx> {
    pub fn new(ctx: &'ctx Context, graph: &CombatGraph) -> Self {
        let mut vars = SatVariables {
            move_hex: HashMap::new(),
            move_time: HashMap::new(),
            attacked: HashMap::new(),
            num_attacks: HashMap::new(),
            removed: HashMap::new(),
            removal_time: HashMap::new(),
        };

        // Create variables for friendly pieces (friend_loc)
        for friend in &graph.friends {
            vars.move_hex.insert(
                *friend,
                Z3Loc::new_const(ctx, format!("move_hex_{}", friend), BV_LOC_SIZE),
            );
            vars.move_time.insert(
                *friend,
                Z3Time::new_const(ctx, format!("move_time_{}", friend), BV_TIME_SIZE),
            );
        }

        // Create variables for all possible attack pairs (attacker_loc, defender_loc)
        for pair in &graph.triples {
            let key = (pair.attacker_pos, pair.defender_pos);
            vars.attacked.insert(
                key,
                Bool::new_const(ctx, format!("attacked_{}{}", key.0, key.1)),
            );
            vars.num_attacks.insert(
                key,
                Z3HP::new_const(ctx, format!("num_attacks_{}{}", key.0, key.1), BV_HP_SIZE),
            );
        }

        // Create variables for all defenders (defender_loc)
        for defender in &graph.defenders {
            vars.removed.insert(
                *defender,
                Bool::new_const(ctx, format!("removed_{}", defender)),
            );
            vars.removal_time.insert(
                *defender,
                Z3Time::new_const(ctx, format!("removal_time_{}", defender), BV_TIME_SIZE),
            );
        }

        vars
    }
}

/// SAT-based combat solver with constraints
pub struct SatCombatSolver<'ctx> {
    pub ctx: &'ctx Context,
    pub solver: Solver<'ctx>,
    pub variables: SatVariables<'ctx>,
    pub graph: CombatGraph,
}

impl<'ctx> SatCombatSolver<'ctx> {
    pub fn new(ctx: &'ctx Context, graph: CombatGraph, board: &Board) -> Self {
        let solver = Solver::new(ctx);
        let variables = SatVariables::new(ctx, &graph);

        let mut sat_solver = SatCombatSolver {
            ctx,
            solver,
            variables,
            graph,
        };

        sat_solver.add_constraints(board);
        sat_solver
    }

    fn add_constraints(&mut self, board: &Board) {
        self.add_movement_constraints(board);
        self.add_attacker_constraints(board);
        self.add_defender_constraints(board);
        self.add_location_constraints(board);
    }

    fn add_movement_constraints(&mut self, board: &Board) {
        // must move to a valid hex
        for friend in &self.graph.friends {
            let valid_move_hexes = board.get_theoretical_move_hexes(*friend);
            self.solver.assert(&loc_var_in_hexes(
                self.ctx,
                &self.variables.move_hex[friend],
                &valid_move_hexes,
            ));
        }

        // no two friendly units can move to the same hex
        let num_friends = self.graph.friends.len();
        for i in 0..num_friends {
            for j in i + 1..num_friends {
                self.solver.assert(
                    &self.variables.move_hex[&self.graph.friends[i]]
                        ._eq(&self.variables.move_hex[&self.graph.friends[j]])
                        .not(),
                );
            }
        }

        // any existing friendly unit must have moved on or before this tick
        // there must be some path to the target hex where all defenders have been removed
        for from_hex in &self.graph.friends {
            for (to_hex, dnf) in &self.graph.move_hex_map[from_hex] {
                if self.graph.friends.contains(to_hex) {
                    self.solver.assert(
                        &self.variables.move_hex[from_hex]
                            ._eq(&crate::core::Loc::as_z3(*to_hex, self.ctx))
                            .implies(
                                &self.variables.move_time[from_hex]
                                    .bvuge(&self.variables.move_time[to_hex]),
                            ),
                    );
                }

                if let Some(dnf) = dnf {
                    let bool_vars: Vec<_> = dnf
                        .iter()
                        .map(|conjunct| {
                            conjunct
                                .iter()
                                .map(|loc| {
                                    if self.graph.defenders.contains(loc) {
                                        Bool::and(
                                            self.ctx,
                                            &[
                                                &self.variables.removed[loc],
                                                &self.variables.move_time[from_hex]
                                                    .bvugt(&self.variables.removal_time[loc]),
                                            ],
                                        )
                                    } else {
                                        Bool::from_bool(self.ctx, false)
                                    }
                                })
                                .collect::<Vec<_>>()
                        })
                        .collect();

                    let bool_vars_refs: Vec<_> = bool_vars
                        .iter()
                        .map(|conjunct| conjunct.iter().collect::<Vec<_>>())
                        .collect();

                    let conjunct_vars = bool_vars_refs
                        .iter()
                        .map(|conjunct| Bool::and(self.ctx, conjunct))
                        .collect::<Vec<_>>();

                    let conjunct_vars_refs: Vec<_> = conjunct_vars.iter().collect();

                    let disjunct = Bool::or(self.ctx, &conjunct_vars_refs);

                    self.solver.assert(
                        &self.variables.move_hex[from_hex]
                            ._eq(&crate::core::Loc::as_z3(*to_hex, self.ctx))
                            .implies(&disjunct),
                    );
                }
            }
        }
    }

    fn add_attacker_constraints(&mut self, board: &Board) {
        for (attacker, defenders) in &self.graph.attacker_to_defenders_map {
            let attacker_piece = board.get_piece(attacker).unwrap();
            let attacker_stats = attacker_piece.unit.stats();

            let attacks_by_this_attacker: Vec<_> = defenders
                .iter()
                .map(|defender| {
                    self.solver.assert(
                        &self.variables.attacked[&(*attacker, *defender)]._eq(
                            &self.variables.num_attacks[&(*attacker, *defender)]
                                .bvugt(&BV::from_u64(self.ctx, 0, BV_HP_SIZE)),
                        ),
                    );
                    &self.variables.attacked[&(*attacker, *defender)]
                })
                .collect();

            let is_attacking = if attacks_by_this_attacker.is_empty() {
                Bool::from_bool(self.ctx, false)
            } else {
                Bool::or(self.ctx, &attacks_by_this_attacker)
            };

            // Lumbering units cannot move and attack in the same turn.
            if attacker_stats.lumbering {
                let moved = self.variables.move_hex[attacker]
                    ._eq(&crate::core::Loc::as_z3(*attacker, self.ctx))
                    .not();
                self.solver.assert(&moved.implies(&is_attacking.not()));
            }

            // Units cannot exceed their number of attacks
            let max_attacks = BV::from_u64(self.ctx, attacker_stats.num_attacks as u64, BV_HP_SIZE);
            let total_attacks: BV<'ctx> = defenders
                .iter()
                .map(|defender| {
                    let num_attacks = &self.variables.num_attacks[&(*attacker, *defender)];
                    // assertion here to prevent overflow solutions
                    self.solver.assert(&num_attacks.bvule(&max_attacks));

                    num_attacks
                })
                .fold(
                    BV::from_u64(self.ctx, 0, BV_HP_SIZE),
                    |acc, attack_count| acc.bvadd(attack_count),
                );

            self.solver.assert(&total_attacks.bvule(&max_attacks));
        }
    }

    fn add_defender_constraints(&mut self, board: &Board) {
        for (defender, attackers) in &self.graph.defender_to_attackers_map {
            let defender_piece = board.get_piece(defender).unwrap();
            let defender_stats = defender_piece.unit.stats();
            let defense = defender_stats.defense;
            let removal_time = &self.variables.removal_time[defender];

            let damages: Vec<_> = attackers
                .iter()
                .filter_map(|attacker| {
                    let attacker_piece = board.get_piece(attacker).unwrap();
                    let attacker_stats = attacker_piece.unit.stats();

                    let damage_value = match attacker_stats.attack {
                        Attack::Damage(damage) => damage,
                        Attack::Unsummon if defender_stats.persistent => 1,
                        Attack::Deathtouch if defender_stats.necromancer => {
                            // disallow this attack
                            self.solver
                                .assert(&self.variables.attacked[&(*attacker, *defender)].not());
                            return None;
                        }
                        _ => defense,
                    };

                    let num_attacks = &self.variables.num_attacks[&(*attacker, *defender)];

                    // force removal time to be after all attacks
                    self.solver.assert(
                        &num_attacks
                            .bvugt(&BV::from_u64(self.ctx, 0, BV_HP_SIZE))
                            .implies(&removal_time.bvuge(&self.variables.move_time[attacker])),
                    );

                    let damage =
                        num_attacks.bvmul(&BV::from_u64(self.ctx, damage_value as u64, BV_HP_SIZE));

                    Some(damage)
                })
                .collect();

            let total_damage = bvsum(self.ctx, &damages);

            let removed = total_damage.bvuge(&BV::from_u64(self.ctx, defense as u64, BV_HP_SIZE));

            self.solver
                .assert(&self.variables.removed[defender]._eq(&removed));
        }
    }

    fn add_location_constraints(&mut self, board: &Board) {
        for CombatTriple {
            attacker_pos,
            defender_pos,
            attack_hexes,
        } in &self.graph.triples
        {
            let is_attacking = &self.variables.attacked[&(*attacker_pos, *defender_pos)];
            let attacker_hex = &self.variables.move_hex[attacker_pos];

            self.solver.assert(&is_attacking.implies(&loc_var_in_hexes(
                self.ctx,
                attacker_hex,
                attack_hexes,
            )));
        }
    }

    pub fn check(&mut self) -> SatResult {
        self.solver.check()
    }

    pub fn get_model(&self) -> Option<Model> {
        self.solver.get_model()
    }

    pub fn assert(&mut self, constraint: &Bool<'ctx>) {
        self.solver.assert(constraint);
    }

    pub fn push(&mut self) {
        self.solver.push();
    }

    pub fn pop(&mut self, n: u32) {
        self.solver.pop(n);
    }
}

/// Generate a sequence of AttackActions from a satisfying SAT model
pub fn generate_move_from_sat_model<'ctx>(
    model: &Model<'ctx>,
    graph: &CombatGraph,
    variables: &SatVariables<'ctx>,
    board: &Board,
) -> Vec<AttackAction> {
    let mut actions = Vec::new();
    let mut moves = HashMap::new();
    let mut move_times = HashMap::new();
    let mut attacks = HashMap::new();

    // First, collect all moves and new positions from the model.
    for friend in &graph.friends {
        let z3_hex_var = &variables.move_hex[friend];
        let val = model.eval(z3_hex_var, true).unwrap().as_u64().unwrap();

        let to_loc = Loc::from_z3(val);
        moves.insert(*friend, to_loc);

        let z3_time_var = &variables.move_time[friend];
        let val = model.eval(z3_time_var, true).unwrap().as_u64().unwrap();

        move_times.insert(*friend, val);
    }

    for ((attacker, defender), num_attacks_var) in &variables.num_attacks {
        let val = model.eval(num_attacks_var, true).unwrap().as_u64().unwrap();

        if val > 0 {
            attacks
                .entry(*attacker)
                .or_insert_with(Vec::new)
                .push((*defender, val));
        }
    }

    let mut movers_by_tick = moves
        .keys()
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
                    if chain_movers.len() > 1 {
                        // not just a self-move
                        actions.push(AttackAction::MoveCyclic {
                            locs: chain_movers.clone(),
                        });
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
                let attack = board
                    .get_piece(&original_attacker)
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

                        let defender_stats = board.get_piece(defender).unwrap().unit.stats();

                        let dmg_val = match attack {
                            Attack::Damage(damage) => damage,
                            Attack::Unsummon if defender_stats.persistent => 1,
                            _ => defender_stats.defense,
                        };

                        // println!("{} attacks {} {} times", original_attacker, defender, num_attacks);
                        for _ in 0..*num_attacks {
                            actions.push(AttackAction::Attack {
                                attacker_loc: *attacker_new_loc,
                                target_loc: *defender,
                            });

                            defender_damage
                                .entry(*defender)
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

fn loc_var_in_hexes<'ctx>(ctx: &'ctx z3::Context, var: &Z3Loc<'ctx>, hexes: &[Loc]) -> Bool<'ctx> {
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

fn bvsum<'ctx>(ctx: &'ctx z3::Context, vars: &[BV<'ctx>]) -> BV<'ctx> {
    vars.iter()
        .fold(BV::from_u64(ctx, 0, BV_HP_SIZE), |acc, var| acc.bvadd(var))
}
