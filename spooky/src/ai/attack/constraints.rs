use std::collections::HashMap;

use z3::{Config, Context, Solver, ast::{Bool, Int, Ast}};
use super::combat::{CombatGraph, CombatPair};

pub struct Variables<'ctx> {
    r_vars: Vec<Bool<'ctx>>,
    tr_vars: Vec<Int<'ctx>>,
    k_vars: Vec<Bool<'ctx>>,
    u_vars: Vec<Bool<'ctx>>,
    p_vars: Vec<Bool<'ctx>>,
    ta_vars: Vec<Int<'ctx>>,
    m_vars: HashMap<CombatPair, Bool<'ctx>>,
    a_vars: Vec<Bool<'ctx>>,
    d_vars: Vec<Int<'ctx>>,
}

pub fn add_constraints<'a>(ctx: &'a Context, solver: &mut Solver, graph: &CombatGraph) -> Variables<'a> {
    // Defender variables
    let mut r_vars = Vec::new();  // removal bools
    let mut tr_vars = Vec::new(); // removal times
    let mut k_vars = Vec::new();  // killed bools
    let mut u_vars = Vec::new();  // unsummoned bools
    
    // Create defender variables
    for _ in graph.defenders.iter() {
        r_vars.push(Bool::fresh_const(ctx, "r"));
        tr_vars.push(Int::fresh_const(ctx, "tr"));
        k_vars.push(Bool::fresh_const(ctx, "k"));
        u_vars.push(Bool::fresh_const(ctx, "u"));
    }

    // Attacker variables
    let mut p_vars = Vec::new();   // passive bools
    let mut ta_vars = Vec::new();  // attack times
    let mut m_vars = HashMap::new();   // move bools

    // Create attacker variables
    for attacker in graph.attackers.iter() {
        p_vars.push(Bool::fresh_const(ctx, "p"));
        ta_vars.push(Int::fresh_const(ctx, "ta"));

        // For each possible attack hex
        for hex in graph.attack_hexes[attacker].iter() {
            m_vars.insert((*attacker, *hex), Bool::fresh_const(ctx, "m"));
        }
    }

    // Combat variables
    let mut a_vars = Vec::new();  // attack bools
    let mut d_vars = Vec::new();  // damage ints

    for _ in graph.pairs.iter() {
        a_vars.push(Bool::fresh_const(ctx, "a"));
        d_vars.push(Int::fresh_const(ctx, "d"));
    }

    // Add constraints
    
    // // Hex constraints - at most one piece per hex
    for hex in graph.hexes.iter() {
        let mut movers = Vec::new();

        for attacker in graph.hex_attackers[hex].iter() {
            movers.push((&m_vars[&(*attacker, *hex)], 1));
        }

        solver.assert(&Bool::pb_le(ctx, &movers, 1));
    }

    todo!();
    // // Movement timing constraints
    // for (i, pair) in pairs.iter().enumerate() {
    //     // ta_x > tr_y for blocking pieces
    //     let ta = &ta_vars[i];
    //     for (j, _) in pairs.iter().enumerate() {
    //         if i != j {
    //             let tr = &tr_vars[j];
    //             solver.assert(&Int::gt(ta, tr));
    //         }
    //     }
    // }

    // // Basic combat constraints
    // for i in 0..pairs.len() {
    //     // Removal implies killed or unsummoned
    //     solver.assert(&Bool::implies(&r_vars[i], 
    //         &Bool::or(ctx, &[&k_vars[i], &u_vars[i]])));
        
    //     // Attack implies not passive
    //     solver.assert(&Bool::implies(&a_vars[i], 
    //         &Bool::not(&p_vars[i])));
    // }
}