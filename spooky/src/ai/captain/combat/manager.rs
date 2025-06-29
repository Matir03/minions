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

use super::graph::{CombatGraph, CombatTriple};

const BV_HP_SIZE: u32 = 10;
type Z3HP<'ctx> = BV<'ctx>;

const BV_TIME_SIZE: u32 = 4;
type Z3Time<'ctx> = BV<'ctx>;

const BV_LOC_SIZE: u32 = 8;
type Z3Loc<'ctx> = BV<'ctx>;

/// Variables for the new SAT-based constraint satisfaction problem
/// Focused on attacking pieces with preprocessed constraints on movement/attack
pub struct SatVariables<'ctx> {
    // Passive variables - only for pieces that could attack
    pub passive: HashMap<Loc, Bool<'ctx>>, // whether the piece is engaged in combat

    // Attack variables - only for non-passive attackers
    pub attack_hex: HashMap<Loc, Z3Loc<'ctx>>, // which hex the attacker moved to before attacking
    pub attack_time: HashMap<Loc, Z3Time<'ctx>>, // on which time the attacker attacked

    // Attack relationship variables
    pub attacked: HashMap<(Loc, Loc), Bool<'ctx>>, // whether the attacker attacked defender
    pub num_attacks: HashMap<(Loc, Loc), Z3HP<'ctx>>, // number of times attacker attacked defender

    // Removal variables
    pub removed: HashMap<Loc, Bool<'ctx>>, // whether defender got removed
    pub removal_time: HashMap<Loc, Z3Time<'ctx>>, // when defender got removed

    // Movement variables for friendly units that need to move out of the way
    // Only included post hoc for friendly units whose movements end up being relevant to combat
    pub move_hex: HashMap<Loc, Z3Loc<'ctx>>, // what hex friendly unit is moving to
    pub move_time: HashMap<Loc, Z3Time<'ctx>>, // when the friendly unit moves

    // Sets of locations
    pub friendly_locs: HashSet<Loc>, // friendly pieces that need to move out of the way (initially empty)
    pub attacker_locs: HashSet<Loc>, // all pieces that could attack some enemy piece
    pub defender_locs: HashSet<Loc>, // all pieces that could be attacked by some enemy piece
}

impl<'ctx> SatVariables<'ctx> {
    pub fn new(ctx: &'ctx Context, graph: &CombatGraph) -> Self {
        let mut vars = SatVariables {
            passive: HashMap::new(),
            attack_hex: HashMap::new(),
            attack_time: HashMap::new(),
            attacked: HashMap::new(),
            num_attacks: HashMap::new(),
            removed: HashMap::new(),
            removal_time: HashMap::new(),
            move_hex: HashMap::new(),
            move_time: HashMap::new(),
            friendly_locs: HashSet::new(),
            attacker_locs: HashSet::new(),
            defender_locs: HashSet::new(),
        };

        // Identify attackers (pieces that could attack some enemy piece)
        for triple in &graph.triples {
            vars.attacker_locs.insert(triple.attacker_pos);
        }

        // Identify defenders (pieces that could be attacked by some enemy piece)
        for defender in &graph.defenders {
            vars.defender_locs.insert(*defender);
        }

        // Create variables for attackers only
        for attacker in &vars.attacker_locs {
            // Passive variable
            vars.passive.insert(
                *attacker,
                Bool::new_const(ctx, format!("passive_{}", attacker)),
            );

            // Attack variables (will be constrained based on passive status)
            vars.attack_hex.insert(
                *attacker,
                Z3Loc::new_const(ctx, format!("attack_hex_{}", attacker), BV_LOC_SIZE),
            );
            vars.attack_time.insert(
                *attacker,
                Z3Time::new_const(ctx, format!("attack_time_{}", attacker), BV_TIME_SIZE),
            );
        }

        // Create variables for all possible attack pairs
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

        // Create variables for all defenders
        for defender in &vars.defender_locs {
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

    /// Add movement variables for a friendly unit that needs to move out of the way
    pub fn add_friendly_movement_variables(&mut self, ctx: &'ctx Context, loc: Loc) {
        if !self.friendly_locs.contains(&loc) {
            self.friendly_locs.insert(loc);
            self.move_hex.insert(
                loc,
                Z3Loc::new_const(ctx, format!("move_hex_{}", loc), BV_LOC_SIZE),
            );
            self.move_time.insert(
                loc,
                Z3Time::new_const(ctx, format!("move_time_{}", loc), BV_TIME_SIZE),
            );
        }
    }
}

/// SAT-based combat solver with constraints
pub struct CombatManager<'ctx> {
    pub ctx: &'ctx Context,
    pub solver: Solver<'ctx>,
    pub variables: SatVariables<'ctx>,
    pub graph: CombatGraph,
}

impl<'ctx> CombatManager<'ctx> {
    pub fn new(ctx: &'ctx Context, graph: CombatGraph, board: &Board) -> Self {
        let solver = Solver::new(ctx);
        let variables = SatVariables::new(ctx, &graph);

        let mut sat_solver = CombatManager {
            ctx,
            solver,
            variables,
            graph,
        };

        sat_solver.add_constraints(board);
        sat_solver
    }

    fn add_constraints(&mut self, board: &Board) {
        self.add_passive_constraints(board);
        self.add_attack_constraints(board);
        self.add_defender_constraints(board);
        self.add_location_constraints(board);
    }

    fn add_passive_constraints(&mut self, board: &Board) {
        // Passive constraint: passive[attacker_loc] => !attacked[attacker_loc][any_defender_loc]
        for attacker in &self.variables.attacker_locs {
            let passive_var = &self.variables.passive[attacker];

            // Get all defenders this attacker could attack
            let empty_vec = Vec::new();
            let defenders = self
                .graph
                .attacker_to_defenders_map
                .get(attacker)
                .unwrap_or(&empty_vec);

            for defender in defenders {
                let attacked_var = &self.variables.attacked[&(*attacker, *defender)];
                // passive => !attacked
                self.solver
                    .assert(&passive_var.implies(&attacked_var.not()));
            }
        }
    }

    fn add_attack_constraints(&mut self, board: &Board) {
        for attacker in &self.variables.attacker_locs {
            let attacker_piece = board.get_piece(attacker).unwrap();
            let attacker_stats = attacker_piece.unit.stats();
            let passive_var = &self.variables.passive[attacker];
            let attack_hex_var = &self.variables.attack_hex[attacker];
            let attack_time_var = &self.variables.attack_time[attacker];

            // Get all defenders this attacker could attack
            let empty_vec = Vec::new();
            let defenders = self
                .graph
                .attacker_to_defenders_map
                .get(attacker)
                .unwrap_or(&empty_vec);

            // Attack hex constraints: must be a possible hex unit could attack from
            // Only apply if not passive
            let mut valid_attack_hexes = Vec::new();
            for defender in defenders {
                for triple in &self.graph.triples {
                    if triple.attacker_pos == *attacker && triple.defender_pos == *defender {
                        valid_attack_hexes.extend(triple.attack_hexes.clone());
                    }
                }
            }

            if !valid_attack_hexes.is_empty() {
                let hex_constraint =
                    loc_var_in_hexes(self.ctx, attack_hex_var, &valid_attack_hexes);
                self.solver
                    .assert(&passive_var.not().implies(&hex_constraint));
            }

            // No two non-passive attackers can have the same attack_hex
            for other_attacker in &self.variables.attacker_locs {
                if other_attacker != attacker {
                    let other_passive_var = &self.variables.passive[other_attacker];
                    let other_attack_hex_var = &self.variables.attack_hex[other_attacker];

                    let same_hex = attack_hex_var._eq(other_attack_hex_var);
                    let both_active =
                        Bool::and(self.ctx, &[&passive_var.not(), &other_passive_var.not()]);
                    self.solver.assert(&both_active.implies(&same_hex.not()));
                }
            }

            // DNF pathing constraints for attack_hex with attack_time as timing variable
            // This is similar to the old movement constraints but for attack positions
            for defender in defenders {
                for triple in &self.graph.triples {
                    if triple.attacker_pos == *attacker && triple.defender_pos == *defender {
                        for attack_hex in &triple.attack_hexes {
                            // Check if this attack hex requires enemy removal
                            if let Some(dnf) =
                                self.get_removal_dnf_for_attack(*attacker, *attack_hex)
                            {
                                let path_constraint = self.create_dnf_constraint(
                                    &dnf,
                                    attack_time_var,
                                    &attack_hex_var._eq(&attack_hex.as_z3(self.ctx)),
                                );
                                self.solver
                                    .assert(&passive_var.not().implies(&path_constraint));
                            }
                        }
                    }
                }
            }

            // Attack relationship constraints
            let attacks_by_this_attacker: Vec<_> = defenders
                .iter()
                .map(|defender| {
                    let attacked_var = &self.variables.attacked[&(*attacker, *defender)];
                    let num_attacks_var = &self.variables.num_attacks[&(*attacker, *defender)];

                    // attacked equals num_attacks > 0
                    self.solver.assert(
                        &attacked_var
                            ._eq(&num_attacks_var.bvugt(&BV::from_u64(self.ctx, 0, BV_HP_SIZE))),
                    );

                    // Must be false if passive
                    self.solver
                        .assert(&passive_var.implies(&attacked_var.not()));

                    attacked_var
                })
                .collect();

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

            // Lumbering units cannot move and attack in the same turn
            if attacker_stats.lumbering {
                // For lumbering units, attack_hex must be the same as current position
                let current_pos = attacker.as_z3(self.ctx);
                let moved = attack_hex_var._eq(&current_pos).not();
                let is_attacking = if attacks_by_this_attacker.is_empty() {
                    Bool::from_bool(self.ctx, false)
                } else {
                    Bool::or(self.ctx, &attacks_by_this_attacker)
                };
                self.solver.assert(&moved.implies(&is_attacking.not()));
            }
        }
    }

    fn add_defender_constraints(&mut self, board: &Board) {
        for defender in &self.variables.defender_locs {
            let defender_piece = board.get_piece(defender).unwrap();
            let defender_stats = defender_piece.unit.stats();
            let defense = defender_stats.defense;
            let removal_time = &self.variables.removal_time[defender];

            let empty_vec = Vec::new();
            let attackers = self
                .graph
                .defender_to_attackers_map
                .get(defender)
                .unwrap_or(&empty_vec);

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

                    // force removal time to be after all attacks (using attack_time, not move_time)
                    let attack_time = &self.variables.attack_time[attacker];
                    self.solver.assert(
                        &num_attacks
                            .bvugt(&BV::from_u64(self.ctx, 0, BV_HP_SIZE))
                            .implies(&removal_time.bvuge(attack_time)),
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
        // Movement constraints for friendly units that need to move out of the way
        for friendly_loc in &self.variables.friendly_locs {
            let move_hex_var = &self.variables.move_hex[friendly_loc];
            let move_time_var = &self.variables.move_time[friendly_loc];

            // Must move to a valid hex
            let valid_move_hexes = board.get_theoretical_move_hexes(*friendly_loc);
            self.solver
                .assert(&loc_var_in_hexes(self.ctx, move_hex_var, &valid_move_hexes));

            // Cannot be the same as the attack_hex of any non-passive attacker
            for attacker in &self.variables.attacker_locs {
                let passive_var = &self.variables.passive[attacker];
                let attack_hex_var = &self.variables.attack_hex[attacker];

                let same_hex = move_hex_var._eq(attack_hex_var);
                let attacker_active = passive_var.not();
                self.solver
                    .assert(&attacker_active.implies(&same_hex.not()));
            }

            // Cannot be the same as the move_hex of any other friendly unit
            for other_friendly in &self.variables.friendly_locs {
                if other_friendly != friendly_loc {
                    let other_move_hex_var = &self.variables.move_hex[other_friendly];
                    self.solver
                        .assert(&move_hex_var._eq(other_move_hex_var).not());
                }
            }

            // DNF pathing constraints with move_time as timing constraint
            for (to_hex, dnf) in &self.graph.move_hex_map[friendly_loc] {
                if let Some(dnf) = dnf {
                    let path_constraint = self.create_dnf_constraint(
                        dnf,
                        move_time_var,
                        &move_hex_var._eq(&to_hex.as_z3(self.ctx)),
                    );
                    self.solver.assert(&path_constraint);
                }
            }
        }
    }

    /// Helper method to get removal DNF for a specific attack position
    fn get_removal_dnf_for_attack(&self, attacker: Loc, attack_hex: Loc) -> Option<Vec<Vec<Loc>>> {
        // This would need to be implemented based on the specific pathfinding logic
        // For now, return None to indicate no removal needed
        None
    }

    /// Helper method to create DNF constraints
    fn create_dnf_constraint<'a>(
        &self,
        dnf: &Vec<Vec<Loc>>,
        timing_var: &Z3Time<'ctx>,
        condition: &Bool<'ctx>,
    ) -> Bool<'ctx> {
        let bool_vars: Vec<_> = dnf
            .iter()
            .map(|conjunct| {
                conjunct
                    .iter()
                    .map(|loc| {
                        if self.variables.defender_locs.contains(loc) {
                            Bool::and(
                                self.ctx,
                                &[
                                    &self.variables.removed[loc],
                                    &timing_var.bvugt(&self.variables.removal_time[loc]),
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

        condition.implies(&disjunct)
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

    // First, collect all moves from the model.
    // This includes both attack moves (for non-passive attackers) and friendly moves (for units that need to move out of the way)

    // Collect attack moves for non-passive attackers
    for attacker in &variables.attacker_locs {
        let passive_var = &variables.passive[attacker];
        let is_passive = model.eval(passive_var, true).unwrap().as_bool().unwrap();

        if is_passive {
            continue;
        }

        // This attacker is not passive, so it has an attack_hex and attack_time
        let attack_hex_var = &variables.attack_hex[attacker];
        let attack_time_var = &variables.attack_time[attacker];

        let hex_val = model.eval(attack_hex_var, true).unwrap().as_u64().unwrap();
        let time_val = model.eval(attack_time_var, true).unwrap().as_u64().unwrap();

        let to_loc = Loc::from_z3(hex_val);
        moves.insert(*attacker, to_loc);
        move_times.insert(*attacker, time_val);

        for defender in &variables.defender_locs {
            let key = (*attacker, *defender);
            if let Some(num_attacks_var) = variables.num_attacks.get(&key) {
                let val = model.eval(num_attacks_var, true).unwrap().as_u64().unwrap();

                if val > 0 {
                    attacks
                        .entry(*attacker)
                        .or_insert_with(Vec::new)
                        .push((*defender, val));
                }
            }
        }
    }

    // Collect moves for friendly units that needed to move out of the way
    for friendly_loc in &variables.friendly_locs {
        // Skip if already processed as an attacker
        if moves.contains_key(friendly_loc) {
            continue;
        }

        let move_hex_var = &variables.move_hex[friendly_loc];
        let move_time_var = &variables.move_time[friendly_loc];

        let hex_val = model.eval(move_hex_var, true).unwrap().as_u64().unwrap();
        let time_val = model.eval(move_time_var, true).unwrap().as_u64().unwrap();

        let to_loc = Loc::from_z3(hex_val);
        moves.insert(*friendly_loc, to_loc);
        move_times.insert(*friendly_loc, time_val);
    }

    // Sort movers by time
    let mut movers_by_tick = moves
        .keys()
        .map(|&mover| (move_times[&mover], mover))
        .collect::<Vec<_>>();

    movers_by_tick.sort_by_key(|&(time, _)| time);

    // Group movers by time
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

    // Process each group of movers
    for mover_group in grouped_movers.iter() {
        let move_actions = group_move_actions(mover_group, &moves);

        actions.extend(move_actions);

        // Then, handle all attacks from units that moved to their attack positions
        for &attacker in mover_group {
            let attacker_attacks = match attacks.get(&attacker) {
                Some(attacks) => attacks,
                None => continue,
            };

            let attacker_pos = moves.get(&attacker).unwrap();
            let attacker_piece = board.get_piece(&attacker).unwrap();
            let attacker_stats = attacker_piece.unit.stats();
            let attack = attacker_stats.attack;

            for (defender, num_attacks) in attacker_attacks {
                if defenders_dead.contains(defender) {
                    continue;
                }

                let defender_stats = board.get_piece(defender).unwrap().unit.stats();

                let dmg_val = match attack {
                    Attack::Damage(damage) => damage,
                    Attack::Unsummon if defender_stats.persistent => 1,
                    _ => defender_stats.defense,
                };

                for _ in 0..*num_attacks {
                    actions.push(AttackAction::Attack {
                        attacker_loc: *attacker_pos,
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

    actions
}

fn group_move_actions<'ctx>(
    mover_group: &HashSet<Loc>,
    moves: &HashMap<Loc, Loc>,
) -> Vec<AttackAction> {
    let mut actions = Vec::new();
    let mut visited = HashSet::new();

    for mover in mover_group {
        if visited.contains(mover) {
            continue;
        }

        let mut chain_movers = Vec::new();
        let mut current_loc = *mover;

        loop {
            chain_movers.push(current_loc);
            visited.insert(current_loc);
            let next_loc = *moves.get(&current_loc).unwrap();

            if next_loc == *mover {
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
