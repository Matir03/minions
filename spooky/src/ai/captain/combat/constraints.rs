use std::collections::HashMap;
use crate::core::{Loc, Board, units::{Attack, UnitStats}};
use super::combat::{CombatGraph, CombatTriple};
use z3::{Context, ast::{Ast, Bool, Int, BV}};

// A helper to convert Loc to a unique i64 for Z3.
fn loc_to_i64(loc: &Loc) -> i64 {
    (loc.x as i64) << 32 | (loc.y as i64 & 0xFFFFFFFF)
}

const BV_SIZE: u32 = 10;
type Z3HP<'ctx> = BV<'ctx>; // BV_SIZE bits

type Z3Time<'ctx> = Int<'ctx>; // for theory of inequality

type Z3Loc<'ctx> = Int<'ctx>;
impl Loc {
    pub fn as_z3<'ctx>(self, ctx: &'ctx Context) -> Z3Loc<'ctx> {
        Int::from_i64(ctx, loc_to_i64(&self))
    }
}

/// Variables for the constraint satisfaction problem, represented as Z3 ASTs.
pub struct Variables<'ctx> {
    // Defender variables
    pub removed: HashMap<Loc, Bool<'ctx>>, // whether a unit is removed
    pub removal_time: HashMap<Loc, Z3Time<'ctx>>, // when the unit is removed
    // pub killed: HashMap<Loc, Bool<'ctx>>,
    // pub unsummoned: HashMap<Loc, Bool<'ctx>>,

    // Movement variables
    pub move_hex: HashMap<Loc, Z3Loc<'ctx>>, // where the unit moves to
    pub move_time: HashMap<Loc, Z3Time<'ctx>>, // when the unit moves and attacks

    // Combat variables
    pub attacks: HashMap<(Loc, Loc), Bool<'ctx>>, // for each (attacker, defender), whether attacker attacks defender
    pub num_attacks: HashMap<(Loc, Loc), Z3HP<'ctx>>, // for each (attacker, defender), number of attacks from attacker to defender
}

impl<'ctx> Variables<'ctx> {
    pub fn new(ctx: &'ctx z3::Context, graph: &CombatGraph) -> Self {
        let mut vars = Variables {
            removed: HashMap::new(),
            removal_time: HashMap::new(),
            // killed: HashMap::new(),
            // unsummoned: HashMap::new(),
            move_time: HashMap::new(),
            move_hex: HashMap::new(),
            attacks: HashMap::new(),
            num_attacks: HashMap::new(),
        };

        for defender in &graph.defenders {
            vars.removed.insert(*defender, Bool::new_const(ctx, format!("removed_{}", loc_to_i64(defender))));
            vars.removal_time.insert(*defender, Int::new_const(ctx, format!("removal_time_{}", loc_to_i64(defender))));
            // vars.killed.insert(*defender, Bool::new_const(ctx, format!("killed_{}", loc_to_i64(defender))));
            // vars.unsummoned.insert(*defender, Bool::new_const(ctx, format!("unsummoned_{}", loc_to_i64(defender))));
        }

        for attacker in &graph.friends {
            vars.move_time.insert(*attacker, Int::new_const(ctx, format!("attack_time_{}", loc_to_i64(attacker))));
            vars.move_hex.insert(*attacker, Int::new_const(ctx, format!("attack_hex_{}", loc_to_i64(attacker))));
        }

        for pair in &graph.triples {
            let key = (pair.attacker_pos, pair.defender_pos);
            vars.attacks.insert(key, Bool::new_const(ctx, format!("attack_{}_{}", loc_to_i64(&key.0), loc_to_i64(&key.1))));
            vars.num_attacks.insert(key, Z3HP::new_const(ctx, format!("damage_{}_{}", loc_to_i64(&key.0), loc_to_i64(&key.1)), BV_SIZE));
        }

        vars
    }
}


fn loc_var_in_hexes<'ctx>(
    ctx: &'ctx z3::Context,
    var: &Int<'ctx>,
    hexes: &[Loc],
) -> Bool<'ctx> {
    if hexes.is_empty() {
        return Bool::from_bool(ctx, false);
    }

    let alternatives = hexes.iter()
        .map(|hex| hex.as_z3(ctx)._eq(var))
        .collect::<Vec<_>>();

    let alternatives_ref: Vec<_> = alternatives.iter().collect();

    Bool::or(ctx, &alternatives_ref)
}

fn bvsum<'ctx>(
    ctx: &'ctx z3::Context,
    vars: &[BV<'ctx>],
) -> BV<'ctx> {
    vars.iter()
        .fold(BV::from_u64(ctx, 0, BV_SIZE), 
            |acc, var| acc.bvadd(var))
}

/// Ensures that friendly units move to valid hexes and that no two units end up in the same hex.
fn add_movement_constraints<'ctx>(
    solver: &z3::Optimize<'ctx>,
    ctx: &'ctx z3::Context,
    variables: &Variables<'ctx>,
    graph: &CombatGraph,
    board: &Board,
) {
    // must move to a valid hex
    for friend in &graph.friends {
        let valid_move_hexes = board.get_theoretical_move_hexes(*friend);
        solver.assert(&loc_var_in_hexes(ctx, &variables.move_hex[friend], &valid_move_hexes));
    }
    
    // no two friendly units can move to the same hex
    let num_friends = graph.friends.len();
    for i in 0..num_friends {
        for j in i + 1..num_friends {
            solver.assert(&variables.move_hex[&graph.friends[i]]._eq(&variables.move_hex[&graph.friends[j]]).not());
        }
    }

    // any existing friendly unit must have moved on or before this tick
    // there must be some path to the target hex where all defenders have been removed
    for from_hex in &graph.friends {
        for (to_hex, dnf) in &graph.move_hex_map[from_hex] {
            if graph.friends.contains(to_hex) {
                solver.assert(&variables.move_hex[from_hex]._eq(&to_hex.as_z3(ctx))
                    .implies(&variables.move_time[from_hex].ge(&variables.move_time[to_hex])));
            }

            if let Some(dnf) = dnf {
                let bool_vars: Vec<_> = dnf.iter()
                    .map(|conjunct|
                        conjunct.iter()
                            .map(|loc| Bool::and(ctx, &[
                                &variables.removed[loc],
                                &variables.move_time[from_hex].gt(&variables.removal_time[loc])
                            ]))
                            .collect::<Vec<_>>()
                        )
                    .collect();

                let bool_vars_refs: Vec<_> = bool_vars.iter()
                    .map(|conjunct| conjunct.iter().collect::<Vec<_>>())
                    .collect();

                let conjunct_vars = bool_vars_refs.iter()
                    .map(|conjunct| Bool::and(ctx, conjunct))
                    .collect::<Vec<_>>();

                let conjunct_vars_refs: Vec<_> = conjunct_vars.iter().collect();

                let disjunct = Bool::or(ctx, &conjunct_vars_refs);

                solver.assert(&variables.move_hex[from_hex]._eq(&to_hex.as_z3(ctx)).implies(&disjunct));
            }
        }
    }
}

/// Ensures that attacks made by attackers follow the constraints of the unit
fn add_attacker_constraints<'ctx>(
    solver: &z3::Optimize<'ctx>,
    ctx: &'ctx z3::Context,
    variables: &Variables<'ctx>,
    graph: &CombatGraph,
    board: &Board,
) {
    for (attacker, defenders) in &graph.attacker_to_defenders_map {
        let attacker_piece = board.get_piece(attacker).unwrap();
        let attacker_stats = attacker_piece.unit.stats();

        let attacks_by_this_attacker: Vec<_> = defenders.iter()
            .map(|defender| {
                solver.assert(
                    &variables.attacks[&(*attacker, *defender)]._eq(
                        &variables.num_attacks[&(*attacker, *defender)].bvugt(
                            &BV::from_u64(ctx, 0, BV_SIZE)
                        )
                    )
                );
                &variables.attacks[&(*attacker, *defender)]
            })
            .collect();

        let is_attacking = if attacks_by_this_attacker.is_empty() {
            Bool::from_bool(ctx, false)
        } else {
            Bool::or(ctx, &attacks_by_this_attacker)
        };

        // Lumbering units cannot move and attack in the same turn.
        if attacker_stats.lumbering {
            let moved = variables.move_hex[attacker]
                ._eq(&attacker.as_z3(ctx))
                .not();
            solver.assert(&moved.implies(&is_attacking.not()));
        }

        // Units cannot exceed their number of attacks
        let max_attacks = BV::from_u64(ctx, attacker_stats.num_attacks as u64, BV_SIZE);
        let total_attacks: BV<'ctx> = defenders.iter()
            .map(|defender| {
                let num_attacks = &variables.num_attacks[&(*attacker, *defender)];
                // assertion here to prevent overflow solutions
                solver.assert(&num_attacks.bvule(&max_attacks));
                
                num_attacks
            })
            .fold(BV::from_u64(ctx, 0, BV_SIZE), |acc, attack_count| acc.bvadd(attack_count));

        solver.assert(&total_attacks.bvule(&max_attacks));   
    }
}

/// Constraint the removal and removal time of defenders
fn add_defender_constraints<'ctx>(
    solver: &z3::Optimize<'ctx>,
    ctx: &'ctx z3::Context,
    variables: &Variables<'ctx>,
    graph: &CombatGraph,
    board: &Board,
) {
    for (defender, attackers) in &graph.defender_to_attackers_map {
        let defender_piece = board.get_piece(defender).unwrap();
        let defender_stats = defender_piece.unit.stats();
        let defense = defender_stats.defense;
        let removal_time = &variables.removal_time[defender];

        let damages: Vec<_> = attackers.iter()
            .filter_map(|attacker| {
                let attacker_piece = board.get_piece(attacker).unwrap();
                let attacker_stats = attacker_piece.unit.stats();
                
                let damage_value = match attacker_stats.attack {
                    Attack::Damage(damage) => damage,
                    Attack::Unsummon if defender_stats.persistent => 1,
                    Attack::Deathtouch if defender_stats.necromancer => {
                        // disallow this attack
                        solver.assert(&variables.attacks[&(*attacker, *defender)].not());
                        return None;
                    },
                    _ => defense,
                };

                let num_attacks = &variables.num_attacks[&(*attacker, *defender)];

                // force removal time to be after all attacks
                solver.assert(&num_attacks.bvugt(&BV::from_u64(ctx, 0, BV_SIZE))
                    .implies(&removal_time.ge(&variables.move_time[attacker]))
                );

                let damage = num_attacks.bvmul(&BV::from_u64(ctx, damage_value as u64, BV_SIZE));

                Some(damage)
            })
            .collect();

        let total_damage = bvsum(ctx, &damages);

        let removed = total_damage.bvuge(&BV::from_u64(ctx, defense as u64, BV_SIZE));

        solver.assert(&variables.removed[defender]._eq(&removed));
    }
}

/// Ensures that attackers attack from range
fn add_location_constraints<'ctx>(
    solver: &z3::Optimize<'ctx>,
    ctx: &'ctx z3::Context,
    variables: &Variables<'ctx>,
    graph: &CombatGraph,
    board: &Board,
) {
    for CombatTriple { attacker_pos, defender_pos, attack_hexes } in &graph.triples {
        let is_attacking = &variables.attacks[&(*attacker_pos, *defender_pos)];
        let attacker_hex = &variables.move_hex[attacker_pos];

        solver.assert(&is_attacking.implies(
            &loc_var_in_hexes(ctx, attacker_hex, attack_hexes)
        ));
    }
}


/// Adds all constraints for the combat graph to the Z3 solver
pub fn add_all_constraints<'ctx>(
    solver: &z3::Optimize<'ctx>,
    ctx: &'ctx z3::Context,
    variables: &Variables<'ctx>,
    graph: &CombatGraph,
    board: &Board,
) {
    add_movement_constraints(solver, ctx, variables, graph, board);
    add_attacker_constraints(solver, ctx, variables, graph, board);
    add_defender_constraints(solver, ctx, variables, graph, board);
    add_location_constraints(solver, ctx, variables, graph, board);

    // let zero = BV::from_u64(ctx, 0, BV_SIZE);
    // let one = BV::from_u64(ctx, 1, BV_SIZE);

    // let num_units_removed: BV<'ctx> = graph.defenders.iter()
    //     .map(|defender| &variables.removed[defender])
    //     .fold(BV::from_u64(ctx, 0, BV_SIZE), |acc, removed| 
    //         acc.bvadd(&removed.ite(&one, &zero)));

    // solver.maximize(&num_units_removed);
}