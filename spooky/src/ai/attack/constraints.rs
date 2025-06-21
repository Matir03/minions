use std::collections::HashMap;
use crate::core::{Loc, Board, units::{Attack, UnitStats}};
use super::combat::CombatGraph;
use z3::ast::{Ast, Bool, Int};

// A helper to convert Loc to a unique i64 for Z3.
fn loc_to_i64(loc: &Loc) -> i64 {
    (loc.x as i64) << 32 | (loc.y as i64 & 0xFFFFFFFF)
}

/// Variables for the constraint satisfaction problem, represented as Z3 ASTs.
pub struct Variables<'ctx> {
    // Defender variables
    pub removed: HashMap<Loc, Bool<'ctx>>,
    pub removal_time: HashMap<Loc, Int<'ctx>>,
    pub killed: HashMap<Loc, Bool<'ctx>>,
    pub unsummoned: HashMap<Loc, Bool<'ctx>>,

    // Attacker variables
    pub passive: HashMap<Loc, Bool<'ctx>>,
    pub attack_time: HashMap<Loc, Int<'ctx>>,
    pub attack_hex: HashMap<Loc, Int<'ctx>>,

    // Combat variables
    pub attacks: HashMap<(Loc, Loc), Bool<'ctx>>,
    pub damage: HashMap<(Loc, Loc), Int<'ctx>>,
}

impl<'ctx> Variables<'ctx> {
    pub fn new(ctx: &'ctx z3::Context, graph: &CombatGraph) -> Self {
        let mut vars = Variables {
            removed: HashMap::new(),
            removal_time: HashMap::new(),
            killed: HashMap::new(),
            unsummoned: HashMap::new(),
            passive: HashMap::new(),
            attack_time: HashMap::new(),
            attack_hex: HashMap::new(),
            attacks: HashMap::new(),
            damage: HashMap::new(),
        };

        for defender in &graph.defenders {
            vars.removed.insert(*defender, Bool::new_const(ctx, format!("removed_{}", loc_to_i64(defender))));
            vars.removal_time.insert(*defender, Int::new_const(ctx, format!("removal_time_{}", loc_to_i64(defender))));
            vars.killed.insert(*defender, Bool::new_const(ctx, format!("killed_{}", loc_to_i64(defender))));
            vars.unsummoned.insert(*defender, Bool::new_const(ctx, format!("unsummoned_{}", loc_to_i64(defender))));
        }

        for attacker in &graph.attackers {
            vars.passive.insert(*attacker, Bool::new_const(ctx, format!("passive_{}", loc_to_i64(attacker))));
            vars.attack_time.insert(*attacker, Int::new_const(ctx, format!("attack_time_{}", loc_to_i64(attacker))));
            vars.attack_hex.insert(*attacker, Int::new_const(ctx, format!("attack_hex_{}", loc_to_i64(attacker))));
        }

        for pair in &graph.pairs {
            let key = (pair.attacker_pos, pair.defender_pos);
            vars.attacks.insert(key, Bool::new_const(ctx, format!("attack_{}_{}", loc_to_i64(&key.0), loc_to_i64(&key.1))));
            vars.damage.insert(key, Int::new_const(ctx, format!("damage_{}_{}", loc_to_i64(&key.0), loc_to_i64(&key.1))));
        }

        vars
    }
}


pub trait Asserter<'ctx> {
    fn assert(&self, ast: &Bool<'ctx>);
}

impl<'ctx> Asserter<'ctx> for z3::Solver<'ctx> {
    fn assert(&self, ast: &Bool<'ctx>) {
        z3::Solver::assert(self, ast);
    }
}

impl<'ctx> Asserter<'ctx> for z3::Optimize<'ctx> {
    fn assert(&self, ast: &Bool<'ctx>) {
        z3::Optimize::assert(self, ast);
    }
}


/// Adds all constraints for the combat graph to the Z3 solver
pub fn add_all_constraints<'ctx, S: Asserter<'ctx>>(
    solver: &S,
    ctx: &'ctx z3::Context,
    variables: &Variables<'ctx>,
    graph: &CombatGraph,
    board: &Board,
) {
    // Time constraints: must be non-negative
    for time_var in variables.removal_time.values().chain(variables.attack_time.values()) {
        solver.assert(&time_var.ge(&Int::from_i64(ctx, 0)));
    }

    // --- Hex constraints ---
    for hex in &graph.hexes {
        let hex_val = Int::from_i64(ctx, loc_to_i64(hex));
        let occupants: Vec<_> = graph.attackers.iter()
            .map(|attacker| variables.attack_hex[attacker].clone()._eq(&hex_val).ite(&Int::from_i64(ctx, 1), &Int::from_i64(ctx, 0)))
            .collect();
        if !occupants.is_empty() {
            let occupant_refs: Vec<_> = occupants.iter().collect();
            solver.assert(&Int::add(ctx, &occupant_refs).le(&Int::from_i64(ctx, 1)));
        }
    }

    // --- Attacker constraints ---
    for attacker in &graph.attackers {
        if let Some(attacker_piece) = board.get_piece(attacker) {
            let attacker_stats = attacker_piece.unit.stats();

            // ExactlyOneAction: passive OR move to exactly one hex
            let is_passive = &variables.passive[attacker];
            let no_hex_val = Int::from_i64(ctx, -1);
            let has_hex = &variables.attack_hex[attacker]._eq(&no_hex_val).not();
            solver.assert(&is_passive.xor(has_hex));

            if let Some(possible_hexes) = graph.attack_hexes.get(attacker) {
                let possible_hex_assertions: Vec<_> = possible_hexes.iter()
                    .map(|h| variables.attack_hex[attacker]._eq(&Int::from_i64(ctx, loc_to_i64(h))))
                    .collect();
                let assertion_refs: Vec<_> = possible_hex_assertions.iter().collect();
                solver.assert(&has_hex.implies(&Bool::or(ctx, &assertion_refs)));
            } else {
                solver.assert(is_passive);
            }

            if attacker_stats.lumbering {
                let moved = variables.attack_hex[attacker]._eq(&Int::from_i64(ctx, loc_to_i64(attacker))).not();
                let attacks_by_this_attacker: Vec<_> = graph.pairs.iter()
                    .filter(|p| p.attacker_pos == *attacker)
                    .map(|p| &variables.attacks[&(p.attacker_pos, p.defender_pos)])
                    .collect();
                if !attacks_by_this_attacker.is_empty() {
                    let is_attacking = Bool::or(ctx, &attacks_by_this_attacker);
                    solver.assert(&moved.implies(&is_attacking.not()));
                }
            }

            let attack_count: Vec<_> = graph.pairs.iter()
                .filter(|p| p.attacker_pos == *attacker)
                .map(|p| variables.attacks[&(p.attacker_pos, p.defender_pos)].clone().ite(&Int::from_i64(ctx, 1), &Int::from_i64(ctx, 0)))
                .collect();
            if !attack_count.is_empty() {
                let attack_count_refs: Vec<_> = attack_count.iter().collect();
                solver.assert(&Int::add(ctx, &attack_count_refs).le(&Int::from_i64(ctx, attacker_stats.num_attacks as i64)));
            }
        }
    }

    // --- Combat pair constraints ---
    for pair in &graph.pairs {
        let attacker = pair.attacker_pos;
        let defender = pair.defender_pos;
        if let (Some(attacker_piece), Some(defender_piece)) = (board.get_piece(&attacker), board.get_piece(&defender)) {
            let attacker_stats = attacker_piece.unit.stats();
            let defender_stats = defender_piece.unit.stats();
            let is_attacking = &variables.attacks[&(attacker, defender)];

            solver.assert(&is_attacking.implies(&variables.passive[&attacker].not()));

            if let Some(possible_hexes) = graph.attack_hexes.get(&attacker) {
                 for hex in possible_hexes {
                     let hex_val = Int::from_i64(ctx, loc_to_i64(hex));
                     let attacker_at_hex = variables.attack_hex[&attacker]._eq(&hex_val);
                     let in_range = hex.dist(&defender) <= attacker_stats.range;
                     let combined_assertion = Bool::and(ctx, &[is_attacking, &attacker_at_hex]);
                     solver.assert(&combined_assertion.implies(&Bool::from_bool(ctx, in_range)));
                 }
            }

            solver.assert(&is_attacking.implies(&variables.attack_time[&attacker].le(&variables.removal_time[&defender])));

            let expected_damage = match attacker_stats.attack {
                Attack::Damage(d) => d,
                Attack::Deathtouch => if defender_stats.necromancer { 0 } else { defender_stats.defense },
                Attack::Unsummon => if defender_stats.persistent { 1 } else { 0 },
            };
            let damage_var = &variables.damage[&(attacker, defender)];
            solver.assert(&damage_var._eq(&is_attacking.ite(&Int::from_i64(ctx, expected_damage as i64), &Int::from_i64(ctx, 0))));
        }
    }

    // --- Defender constraints ---
    for defender in &graph.defenders {
        if let Some(defender_piece) = board.get_piece(defender) {
            let defender_stats = defender_piece.unit.stats();

            let fates = [
                (&variables.removed[defender], 1),
                (&variables.killed[defender], 1),
                (&variables.unsummoned[defender], 1),
            ];
            solver.assert(&Bool::pb_le(ctx, &fates, 1));

            let attackers_with_unsummon: Vec<_> = graph.pairs.iter()
                .filter(|p| p.defender_pos == *defender)
                .filter_map(|p| board.get_piece(&p.attacker_pos)
                    .map(|attacker_piece| (p, attacker_piece.unit.stats().attack)))
                .filter(|(_, attack)| matches!(attack, Attack::Unsummon))
                .map(|(p, _)| &variables.attacks[&(p.attacker_pos, p.defender_pos)])
                .collect();
            if !attackers_with_unsummon.is_empty() {
                let is_attacked_by_unsummon = Bool::or(ctx, &attackers_with_unsummon);
                solver.assert(&variables.unsummoned[defender].implies(&is_attacked_by_unsummon));
            } else {
                solver.assert(&variables.unsummoned[defender].not());
            }

            let total_damage_vars: Vec<_> = graph.pairs.iter()
                .filter(|p| p.defender_pos == *defender)
                .map(|p| variables.damage[&(p.attacker_pos, p.defender_pos)].clone())
                .collect();
            if !total_damage_vars.is_empty() {
                let total_damage_refs: Vec<_> = total_damage_vars.iter().collect();
                let total_damage_sum = Int::add(ctx, &total_damage_refs);
                let defense = Int::from_i64(ctx, defender_stats.defense as i64);
                solver.assert(&variables.removed[defender]._eq(&total_damage_sum.ge(&defense)));
                solver.assert(&total_damage_sum.le(&defense)); // NoOverkill
            }
        }
    }
}