use std::collections::{HashMap, HashSet};

use crate::core::board::actions::AttackAction;
use crate::core::board::{Board, Piece};

use crate::core::{
    loc::Loc,
    side::Side,
    units::{Attack, Unit, UnitStats},
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

pub struct Z3Move<'a, 'ctx> {
    pub from: Loc,
    pub to: &'a Z3Loc<'ctx>,
    pub time: &'a Z3Time<'ctx>,
}

/// Variables for the new SAT-based constraint satisfaction problem
/// Focused on attacking pieces with preprocessed constraints on movement/attack
#[derive(Debug)]
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
    // pub removed: HashMap<Loc, Bool<'ctx>>, // whether defender got removed
    pub killed: HashMap<Loc, Bool<'ctx>>, // whether defender got killed
    pub unsummoned: HashMap<Loc, Bool<'ctx>>, // whether defender got unsummoned
    pub removal_time: HashMap<Loc, Z3Time<'ctx>>, // when defender got removed (killed or unsummoned)

    // Movement variables for friendly units that need to move out of the way
    // Only included post hoc for friendly units whose movements end up being relevant to combat
    pub move_hex: HashMap<Loc, Z3Loc<'ctx>>, // what hex friendly unit is moving to
    pub move_time: HashMap<Loc, Z3Time<'ctx>>, // when the friendly unit moves

    // Blink variables
    pub blink: HashMap<Loc, Bool<'ctx>>, // whether an attacker with blink keyword is blinking
}

impl<'ctx> SatVariables<'ctx> {
    pub fn new(ctx: &'ctx Context, graph: &CombatGraph, board: &Board) -> Self {
        let mut vars = SatVariables {
            passive: HashMap::new(),
            attack_hex: HashMap::new(),
            attack_time: HashMap::new(),
            attacked: HashMap::new(),
            num_attacks: HashMap::new(),
            killed: HashMap::new(),
            unsummoned: HashMap::new(),
            removal_time: HashMap::new(),
            move_hex: HashMap::new(),
            move_time: HashMap::new(),
            blink: HashMap::new(),
        };

        // Create variables for attackers only
        for attacker in &graph.attackers {
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

            // Add blink variable if the unit has the blink keyword
            let piece = board.get_piece(attacker).unwrap();
            if piece.unit.stats().blink {
                vars.blink.insert(
                    *attacker,
                    Bool::new_const(ctx, format!("blink_{}", attacker)),
                );
            }
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
        for defender in &graph.defenders {
            vars.killed.insert(
                *defender,
                Bool::new_const(ctx, format!("killed_{}", defender)),
            );
            vars.unsummoned.insert(
                *defender,
                Bool::new_const(ctx, format!("unsummoned_{}", defender)),
            );
            vars.removal_time.insert(
                *defender,
                Z3Time::new_const(ctx, format!("removal_time_{}", defender), BV_TIME_SIZE),
            );
        }

        vars
    }

    pub fn attack_move<'a>(&'a self, attacker: &Loc) -> Z3Move<'a, 'ctx> {
        Z3Move {
            from: *attacker,
            to: &self.attack_hex[attacker],
            time: &self.attack_time[attacker],
        }
    }

    pub fn friend_move<'a>(&'a self, loc: &Loc) -> Z3Move<'a, 'ctx> {
        Z3Move {
            from: *loc,
            to: &self.move_hex[loc],
            time: &self.move_time[loc],
        }
    }

    /// Add movement variables for a friendly unit that needs to move out of the way
    pub fn add_friendly_movement_variable(&mut self, ctx: &'ctx Context, loc: Loc, blink: bool) {
        if self.move_hex.contains_key(&loc) {
            return;
        }

        self.move_hex.insert(
            loc,
            Z3Loc::new_const(ctx, format!("move_hex_{}", loc), BV_LOC_SIZE),
        );
        self.move_time.insert(
            loc,
            Z3Time::new_const(ctx, format!("move_time_{}", loc), BV_TIME_SIZE),
        );

        if blink {
            self.blink
                .insert(loc, Bool::new_const(ctx, format!("blink_{}", loc)));
        }
    }
}

/// SAT-based combat solver with constraints
pub struct ConstraintManager<'ctx> {
    pub ctx: &'ctx Context,
    pub solver: Solver<'ctx>,
    pub variables: SatVariables<'ctx>,
    pub graph: CombatGraph,
}

impl<'ctx> ConstraintManager<'ctx> {
    pub fn new(ctx: &'ctx Context, graph: CombatGraph, board: &Board) -> Self {
        let solver = Solver::new(ctx);
        let variables = SatVariables::new(ctx, &graph, board);

        let mut manager = ConstraintManager {
            ctx,
            solver,
            variables,
            graph,
        };

        manager.add_constraints(board);
        manager
    }

    fn add_constraints(&mut self, board: &Board) {
        self.add_passive_constraints(board);
        self.add_attack_constraints(board);
        self.add_defender_constraints(board);
        self.add_triple_constraints(board);
    }

    fn add_passive_constraints(&mut self, board: &Board) {
        // Passive constraint: passive[attacker_loc] => !attacked[attacker_loc][any_defender_loc]
        for (attacker, defenders) in &self.graph.attacker_to_defenders_map {
            let passive_var = &self.variables.passive[attacker];

            for defender in defenders {
                let attacked_var = &self.variables.attacked[&(*attacker, *defender)];
                // passive => !attacked
                self.solver
                    .assert(&passive_var.implies(&attacked_var.not()));
            }
        }
    }

    pub fn blink_constraint<'a>(
        &self,
        move1: &Z3Move<'a, 'ctx>,
        move2: &Z3Move<'a, 'ctx>,
    ) -> Bool<'ctx> {
        let Z3Move {
            from: loc1,
            to: dest1,
            time: time1,
        } = move1;
        let Z3Move {
            from: loc2,
            to: dest2,
            time: time2,
        } = move2;

        let same_hex = dest1._eq(dest2);

        // If both are active and moving to the same hex, one must be blinking.
        let blink_resolves = match (
            self.variables.blink.get(loc1),
            self.variables.blink.get(loc2),
        ) {
            (Some(blink1), Some(blink2)) => {
                let first = time1.bvult(time2);
                let second = time2.bvult(time1);

                Bool::or(
                    self.ctx,
                    &[
                        &Bool::and(self.ctx, &[&first, blink1]),
                        &Bool::and(self.ctx, &[&second, blink2]),
                    ],
                )
            }
            (Some(blink1), None) => {
                let first = time1.bvult(time2);
                Bool::and(self.ctx, &[&first, blink1])
            }
            (None, Some(blink2)) => {
                let second = time2.bvult(time1);
                Bool::and(self.ctx, &[&second, blink2])
            }
            _ => Bool::from_bool(self.ctx, false),
        };

        same_hex.implies(&blink_resolves)
    }

    fn add_attack_constraints(&mut self, board: &Board) {
        for (attacker, defenders) in &self.graph.attacker_to_defenders_map {
            let attacker_piece = board.get_piece(attacker).unwrap();
            let attacker_stats = attacker_piece.unit.stats();
            let passive_var = &self.variables.passive[attacker];
            let attack_hex_var = &self.variables.attack_hex[attacker];
            let attack_time_var = &self.variables.attack_time[attacker];

            let valid_attack_hexes = self.graph.attack_hex_map.get(attacker).unwrap();
            assert!(!valid_attack_hexes.is_empty());

            let hex_constraint = loc_var_in_hexes(self.ctx, attack_hex_var, &valid_attack_hexes);
            self.solver
                .assert(&passive_var.not().implies(&hex_constraint));

            // No two non-passive attackers can conflict on hex
            for other_attacker in &self.graph.attackers {
                if other_attacker != attacker {
                    let other_passive_var = &self.variables.passive[other_attacker];
                    let both_active =
                        Bool::and(self.ctx, &[&passive_var.not(), &other_passive_var.not()]);

                    let attack_move = self.variables.attack_move(attacker);
                    let other_attack_move = self.variables.attack_move(other_attacker);

                    let blink_constraint = self.blink_constraint(&attack_move, &other_attack_move);
                    self.solver.assert(&both_active.implies(&blink_constraint));
                }
            }

            for attack_hex in valid_attack_hexes {
                let dnf = &self.graph.move_hex_map[attacker][&attack_hex];

                if let Some(dnf) = dnf {
                    let path_constraint = self.create_dnf_constraint(
                        &dnf,
                        attack_time_var,
                        &attack_hex_var._eq(&attack_hex.as_z3(self.ctx)),
                    );
                    self.solver
                        .assert(&passive_var.not().implies(&path_constraint));
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
        for defender in &self.graph.defenders {
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

            let mut unsummons = Vec::new();

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
                        Attack::Unsummon => {
                            unsummons.push(&self.variables.attacked[&(*attacker, *defender)]);
                            return None;
                        }
                        Attack::Deathtouch => defender_stats.defense,
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

            let killed = total_damage.bvuge(&BV::from_u64(self.ctx, defense as u64, BV_HP_SIZE));
            self.solver
                .assert(&self.variables.killed[defender]._eq(&killed));

            let unsummoned = Bool::or(self.ctx, &unsummons);
            self.solver
                .assert(&self.variables.unsummoned[defender]._eq(&unsummoned));

            self.solver
                .assert(&Bool::or(self.ctx, &[&killed.not(), &unsummoned.not()]));
        }
    }

    fn add_triple_constraints(&mut self, board: &Board) {
        for triple in &self.graph.triples {
            let CombatTriple {
                attacker_pos,
                defender_pos,
                attack_hexes,
            } = triple;

            let attack_hex_var = &self.variables.attack_hex[attacker_pos];
            let attack_time_var = &self.variables.attack_time[attacker_pos];

            let attack_hex_constraint = loc_var_in_hexes(self.ctx, attack_hex_var, attack_hexes);
            self.solver.assert(
                &self.variables.attacked[&(*attacker_pos, *defender_pos)]
                    .implies(&attack_hex_constraint),
            );
        }
    }

    /// Helper method to get removal DNF for a specific attack position
    fn get_removal_dnf_for_attack(&self, attacker: Loc, attack_hex: Loc) -> Option<Vec<Vec<Loc>>> {
        // This would need to be implemented based on the specific pathfinding logic
        // For now, return None to indicate no removal needed
        None
    }

    /// Helper method to create DNF constraints
    pub fn create_dnf_constraint<'a>(
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
                        if self.graph.defenders.contains(loc) {
                            Bool::and(
                                self.ctx,
                                &[
                                    &Bool::or(
                                        self.ctx,
                                        &[
                                            &self.variables.killed[loc],
                                            &self.variables.unsummoned[loc],
                                        ],
                                    ),
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

    pub fn untouched(&self, loc: &Loc) -> bool {
        !self.variables.move_hex.contains_key(loc) && !self.graph.attackers.contains(loc)
    }

    pub fn untouched_friendly_units(&self) -> HashSet<Loc> {
        self.graph
            .friends
            .iter()
            .filter(|loc| self.untouched(loc))
            .cloned()
            .collect()
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
    for (attacker, defenders) in &graph.attacker_to_defenders_map {
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

        for defender in defenders {
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
    for (friendly_loc, move_hex_var) in &variables.move_hex {
        // Skip if already processed as an attacker
        if moves.contains_key(friendly_loc) {
            continue;
        }

        let move_time_var = &variables.move_time[friendly_loc];

        let hex_val = model.eval(move_hex_var, true).unwrap().as_u64().unwrap();
        let time_val = model.eval(move_time_var, true).unwrap().as_u64().unwrap();

        let to_loc = Loc::from_z3(hex_val);
        moves.insert(*friendly_loc, to_loc);
        move_times.insert(*friendly_loc, time_val);

        // Add blink if the unit is blinking
        if let Some(blink_var) = variables.blink.get(friendly_loc) {
            let blink_val = model.eval(blink_var, true).unwrap().as_bool().unwrap();
            if blink_val {
                actions.push(AttackAction::Blink {
                    blink_loc: *friendly_loc,
                });
            }
        }
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
                None => &vec![],
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

            // Add blink if the unit is blinking
            if let Some(blink_var) = variables.blink.get(&attacker) {
                let blink_val = model.eval(blink_var, true).unwrap().as_bool().unwrap();
                if blink_val {
                    actions.push(AttackAction::Blink {
                        blink_loc: *attacker_pos,
                    });
                }
            }
        }
    }

    actions
}

// TODO: attacks need to be interspersed with moves and blinks
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
