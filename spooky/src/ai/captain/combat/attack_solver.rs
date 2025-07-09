use anyhow::{bail, Context as AnyhowContext, Result};
use lapjv::lapjv;
use std::collections::{HashMap, HashSet};
use z3::{
    ast::{Ast, Bool, BV},
    Context, Model, SatResult, Solver,
};

use crate::core::{
    board::{actions::AttackAction, Board},
    loc::Loc,
    side::Side,
    units::{Attack, Unit},
};

use super::{constraints::SatVariables, graph::CombatGraph, prophet::DeathProphet};

const BV_TIME_SIZE: u32 = 4;
type Z3Time<'ctx> = BV<'ctx>;

const BV_LOC_SIZE: u32 = 8;
type Z3Loc<'ctx> = BV<'ctx>;

/// Represents a movement segment in the attack phase
#[derive(Debug, Clone)]
pub struct MovementSegment {
    pub attacker_loc: Loc,
    pub destinations: Vec<Loc>,
    pub attack_time: u64,
}

/// Represents a cycle in the movement graph
#[derive(Debug, Clone)]
pub struct MovementCycle {
    pub locs: Vec<Loc>,
    pub attack_time: u64,
}

/// New attack solver that uses death prophet priorities and optimal matching
pub struct AttackSolver<'ctx> {
    ctx: &'ctx Context,
    solver: Solver<'ctx>,
    variables: SatVariables<'ctx>,
    graph: CombatGraph,
    death_prophet: DeathProphet,

    // Precomputed DNFs for each move
    move_dnfs: HashMap<(Loc, Loc), Vec<Vec<Loc>>>,

    // Track which moves are forbidden
    cant_move: HashSet<(Loc, Loc)>,

    // Track which assumptions are unassumable
    unassumable_remove: HashSet<Loc>,
    unassumable_keep: HashSet<Loc>,
}

impl<'ctx> AttackSolver<'ctx> {
    pub fn new(
        ctx: &'ctx Context,
        graph: CombatGraph,
        board: &Board,
        death_prophet: DeathProphet,
    ) -> Self {
        let solver = Solver::new(ctx);
        let mut variables = SatVariables::new(ctx, &graph);

        // Add movement variables for all friendly units
        for friend in &graph.friends {
            variables.add_friendly_movement_variable(ctx, *friend);
        }

        // Precompute DNFs for each move
        let mut move_dnfs = HashMap::new();
        for (friend, hex_map) in &graph.move_hex_map {
            for (dest, dnf) in hex_map {
                if let Some(dnf_clauses) = dnf {
                    move_dnfs.insert((*friend, *dest), dnf_clauses.clone());
                }
            }
        }

        Self {
            ctx,
            solver,
            variables,
            graph,
            death_prophet,
            move_dnfs,
            cant_move: HashSet::new(),
            unassumable_remove: HashSet::new(),
            unassumable_keep: HashSet::new(),
        }
    }

    /// Generate attack actions using the new strategy
    pub fn generate_attacks(&mut self, board: &Board, side: Side) -> Result<Vec<AttackAction>> {
        // Step 1: Generate priorities for all assumptions
        let all_priorities = self.death_prophet.generate_assumptions(board, side);
        println!("[AttackSolver] Initial move priorities:");
        for p in &all_priorities {
            if let super::prophet::RemovalAssumption::Move(from, to) = p {
                println!("  Move from {:?} to {:?}", from, to);
            }
        }
        self.precompute_dnfs();
        self.cant_move.clear();
        self.unassumable_remove.clear();
        self.unassumable_keep.clear();
        let mut iter_count = 0;
        let mut printed_cost_matrix = false;

        // Step 2: Iteratively solve
        loop {
            iter_count += 1;
            // Filter priorities to exclude unassumable removes/keeps and cant_move moves
            let priorities: Vec<_> = all_priorities
                .iter()
                .filter(|assumption| match assumption {
                    super::prophet::RemovalAssumption::Removed(loc) => {
                        !self.unassumable_remove.contains(loc)
                    }
                    super::prophet::RemovalAssumption::NotRemoved(loc) => {
                        !self.unassumable_keep.contains(loc)
                    }
                    super::prophet::RemovalAssumption::Move(from, to) => {
                        !self.cant_move.contains(&(*from, *to))
                    }
                })
                .cloned()
                .collect();
            println!(
                "[AttackSolver] Iteration {}: priorities left: {}",
                iter_count,
                priorities.len()
            );
            if priorities.is_empty() {
                println!("[AttackSolver] No priorities left, returning empty actions");
                return Ok(vec![]);
            }
            // 2a: Compute optimal matching using LAPJV
            if !printed_cost_matrix {
                self.print_cost_matrix(&priorities);
                printed_cost_matrix = true;
            }
            let matching = self.compute_optimal_matching(&priorities)?;
            if matching.is_none() {
                println!("[AttackSolver] No matching found, removing lowest-priority assumption");
                self.remove_lowest_priority_assumption(&[], &[], &priorities);
                continue;
            }
            let matching = matching.unwrap();
            println!("[AttackSolver] Matching found");

            // 2b: Identify paths and cycles in the movement graph
            let (paths, cycles) = self.identify_paths_and_cycles(&matching);

            // 2c: Compute move time constraints and build SAT assumptions
            let assumptions = self.compute_move_time_constraints(&paths, &cycles, &priorities);

            // 2d: Query SAT with assumptions
            let result = self.query_sat_with_assumptions(&assumptions)?;
            match result {
                SatResult::Sat => {
                    println!("[AttackSolver] SAT: extracting model");
                    let model = self.solver.get_model().context("Failed to get model")?;
                    return self.convert_model_to_actions(&model, &paths, &cycles, board);
                }
                SatResult::Unsat => {
                    println!(
                        "[AttackSolver] UNSAT: removing lowest-priority assumption from unsat core"
                    );
                    let unsat_core = self.solver.get_unsat_core();
                    self.remove_lowest_priority_assumption(&unsat_core, &assumptions, &priorities);
                }
                SatResult::Unknown => {
                    bail!(
                        "Unknown result from SAT solver: {:?}",
                        self.solver.get_reason_unknown().unwrap()
                    );
                }
            }
        }
    }

    /// Precompute DNFs for each move
    fn precompute_dnfs(&mut self) {
        // DNFs are already computed in the constructor
        // This method can be used for additional preprocessing if needed
    }

    /// Compute optimal matching using LAPJV algorithm
    fn compute_optimal_matching(
        &mut self,
        priorities: &[super::prophet::RemovalAssumption],
    ) -> Result<Option<Vec<usize>>> {
        // Get all possible destinations from move_hex_map
        let mut all_destinations = HashSet::new();
        for (_, hex_map) in &self.graph.move_hex_map {
            for (dest, _) in hex_map {
                all_destinations.insert(*dest);
            }
        }

        let friend_locs: Vec<Loc> = self.graph.friends.iter().cloned().collect();
        let destination_locs: Vec<Loc> = all_destinations.into_iter().collect();

        if friend_locs.is_empty() || destination_locs.is_empty() {
            return Ok(Some(Vec::new()));
        }

        // Build cost matrix: friend_locs x destination_locs
        let mut cost_matrix = vec![vec![f64::INFINITY; destination_locs.len()]; friend_locs.len()];

        for (friend_idx, &friend_loc) in friend_locs.iter().enumerate() {
            if let Some(hex_map) = self.graph.move_hex_map.get(&friend_loc) {
                for (dest_idx, &dest_loc) in destination_locs.iter().enumerate() {
                    // Only allow moves present in priorities
                    let move_allowed = priorities.iter().any(|ass| match ass {
                        super::prophet::RemovalAssumption::Move(f, t) => {
                            *f == friend_loc && *t == dest_loc
                        }
                        _ => false,
                    });
                    if !move_allowed {
                        cost_matrix[friend_idx][dest_idx] = f64::INFINITY;
                        continue;
                    }
                    if self.cant_move.contains(&(friend_loc, dest_loc)) {
                        cost_matrix[friend_idx][dest_idx] = f64::INFINITY;
                        continue;
                    }
                    if let Some(dnf) = hex_map.get(&dest_loc) {
                        let distance = friend_loc.dist(&dest_loc) as f64;
                        let move_prob = if dnf.is_none() { 1.0 } else { 0.5 };
                        if move_prob > 0.0 {
                            cost_matrix[friend_idx][dest_idx] =
                                -(move_prob as f64).log2() + distance * 0.1;
                        } else {
                            cost_matrix[friend_idx][dest_idx] = f64::INFINITY;
                        }
                    } else {
                        cost_matrix[friend_idx][dest_idx] = f64::INFINITY;
                    }
                }
            }
        }

        use ndarray::Array2;

        println!("=== LAPJV DEBUG ===");
        println!(
            "Cost matrix shape: {}x{}",
            friend_locs.len(),
            destination_locs.len()
        );
        println!("Friend locations: {:?}", friend_locs);
        println!("Destination locations: {:?}", destination_locs);

        // Print cost matrix for debugging
        for (i, row) in cost_matrix.iter().enumerate() {
            println!("Friend {} ({:?}) costs: {:?}", i, friend_locs[i], row);
        }

        // LAPJV requires a square matrix. We need to make it square by either:
        // 1. Adding dummy destinations (if more friends than destinations)
        // 2. Adding dummy friends (if more destinations than friends)
        // 3. Using a different algorithm for rectangular matrices

        let max_dim = friend_locs.len().max(destination_locs.len());
        let mut square_cost_matrix = vec![vec![f64::INFINITY; max_dim]; max_dim];

        // Copy the original cost matrix
        for (i, row) in cost_matrix.iter().enumerate() {
            for (j, &cost) in row.iter().enumerate() {
                square_cost_matrix[i][j] = cost;
            }
        }

        // Fill remaining rows/columns with high costs (dummy assignments)
        for i in friend_locs.len()..max_dim {
            for j in 0..max_dim {
                square_cost_matrix[i][j] = f64::INFINITY;
            }
        }
        for j in destination_locs.len()..max_dim {
            for i in 0..max_dim {
                square_cost_matrix[i][j] = f64::INFINITY;
            }
        }

        println!("Square cost matrix shape: {}x{}", max_dim, max_dim);

        // Clone the cost matrix for the array conversion
        let cost_array_data: Vec<f64> = square_cost_matrix.iter().flatten().cloned().collect();
        let cost_array = Array2::from_shape_vec((max_dim, max_dim), cost_array_data).unwrap();

        match lapjv(&cost_array) {
            Ok((row_indices, col_indices)) => {
                println!("LAPJV succeeded!");
                println!("Row indices (friend -> dest): {:?}", row_indices);
                println!("Col indices: {:?}", col_indices);

                let mut matching = Vec::new();
                for (friend_idx, &dest_idx) in row_indices.iter().enumerate() {
                    // Only consider real friends and real destinations
                    if friend_idx < friend_locs.len() && dest_idx < destination_locs.len() {
                        // Check if this is a valid assignment (not dummy)
                        if square_cost_matrix[friend_idx][dest_idx] < f64::INFINITY {
                            matching.push(dest_idx);
                            println!(
                                "Matched friend {} ({:?}) to destination {:?}",
                                friend_idx, friend_locs[friend_idx], destination_locs[dest_idx]
                            );
                        } else {
                            println!(
                                "Friend {} ({:?}) assigned to invalid destination (dummy)",
                                friend_idx, friend_locs[friend_idx]
                            );
                        }
                    }
                }
                println!("Final matching: {:?}", matching);
                Ok(Some(matching))
            }
            Err(e) => {
                println!("LAPJV failed with error: {:?}", e);
                Ok(None)
            }
        }
    }

    /// Identify paths and cycles in the movement graph
    fn identify_paths_and_cycles(
        &self,
        matching: &[usize],
    ) -> (Vec<MovementSegment>, Vec<MovementCycle>) {
        let mut paths = Vec::new();
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();

        let friend_locs: Vec<Loc> = self.graph.friends.iter().cloned().collect();

        // Get all possible destinations from move_hex_map
        let mut all_destinations = HashSet::new();
        for (_, hex_map) in &self.graph.move_hex_map {
            for (dest, _) in hex_map {
                all_destinations.insert(*dest);
            }
        }
        let destination_locs: Vec<Loc> = all_destinations.into_iter().collect();

        for (friend_idx, &dest_idx) in matching.iter().enumerate() {
            if friend_idx >= friend_locs.len() || dest_idx >= destination_locs.len() {
                continue;
            }

            let friend_loc = friend_locs[friend_idx];
            let dest_loc = destination_locs[dest_idx];

            if visited.contains(&friend_loc) {
                continue;
            }

            // Create a simple path from friend to destination
            paths.push(MovementSegment {
                attacker_loc: friend_loc,
                destinations: vec![dest_loc],
                attack_time: 0, // Will be set later
            });

            visited.insert(friend_loc);
        }

        (paths, cycles)
    }

    /// Compute move time constraints for the movement graph
    fn compute_move_time_constraints(
        &self,
        paths: &[MovementSegment],
        cycles: &[MovementCycle],
        priorities: &[super::prophet::RemovalAssumption],
    ) -> Vec<Bool<'ctx>> {
        let mut assumptions = Vec::new();

        // Add constraints for paths
        for path in paths {
            for i in 0..path.destinations.len() - 1 {
                let from = path.destinations[i];
                let to = path.destinations[i + 1];

                if let Some(dnf) = self.move_dnfs.get(&(from, to)) {
                    // Find lowest priority move in the path
                    let lowest_priority =
                        self.find_lowest_priority_in_path(&path.destinations[..i + 2], priorities);

                    // Create DNF constraint
                    let constraint = self.create_dnf_constraint(
                        dnf,
                        &self.variables.move_time[&from],
                        &lowest_priority,
                    );
                    assumptions.push(constraint);
                }
            }
        }

        // Add constraints for cycles
        for cycle in cycles {
            for i in 0..cycle.locs.len() {
                let from = cycle.locs[i];
                let to = cycle.locs[(i + 1) % cycle.locs.len()];

                if let Some(dnf) = self.move_dnfs.get(&(from, to)) {
                    let lowest_priority =
                        self.find_lowest_priority_in_cycle(&cycle.locs, priorities);
                    let constraint = self.create_dnf_constraint(
                        dnf,
                        &self.variables.move_time[&from],
                        &lowest_priority,
                    );
                    assumptions.push(constraint);
                }
            }
        }

        // Add assumable remove/keep constraints for each defender
        for defender in &self.graph.defenders {
            if !self.unassumable_remove.contains(defender) {
                assumptions.push(self.variables.removed[defender].clone());
            }
            if !self.unassumable_keep.contains(defender) {
                assumptions.push(self.variables.removed[defender].not());
            }
        }

        assumptions
    }

    /// Find lowest priority assumption in a path
    fn find_lowest_priority_in_path(
        &self,
        path: &[Loc],
        priorities: &[super::prophet::RemovalAssumption],
    ) -> Bool<'ctx> {
        // For now, return a simple assumption
        // In a real implementation, this would find the actual lowest priority
        Bool::from_bool(self.ctx, true)
    }

    /// Find lowest priority assumption in a cycle
    fn find_lowest_priority_in_cycle(
        &self,
        cycle: &[Loc],
        priorities: &[super::prophet::RemovalAssumption],
    ) -> Bool<'ctx> {
        // For now, return a simple assumption
        Bool::from_bool(self.ctx, true)
    }

    /// Create DNF constraint
    fn create_dnf_constraint(
        &self,
        dnf: &[Vec<Loc>],
        timing_var: &Z3Time<'ctx>,
        condition: &Bool<'ctx>,
    ) -> Bool<'ctx> {
        let mut clauses = Vec::new();

        for clause in dnf {
            let mut literals = Vec::new();
            for &loc in clause {
                let removed_var = &self.variables.removed[&loc];
                literals.push(removed_var.clone());
            }

            if !literals.is_empty() {
                let clause_bool = Bool::and(self.ctx, &literals.iter().collect::<Vec<_>>());
                clauses.push(clause_bool);
            }
        }

        if clauses.is_empty() {
            Bool::from_bool(self.ctx, true)
        } else {
            let dnf_bool = Bool::or(self.ctx, &clauses.iter().collect::<Vec<_>>());
            condition.implies(&dnf_bool)
        }
    }

    /// Query SAT solver with assumptions
    fn query_sat_with_assumptions(&mut self, assumptions: &[Bool<'ctx>]) -> Result<SatResult> {
        Ok(self.solver.check_assumptions(assumptions))
    }

    /// Remove lowest priority assumption from unsat core
    fn remove_lowest_priority_assumption(
        &mut self,
        unsat_core: &[Bool<'ctx>],
        assumptions: &[Bool<'ctx>],
        priorities: &[super::prophet::RemovalAssumption],
    ) {
        // Find the lowest priority assumption in the unsat core
        // For now, just remove a random assumption
        if let Some(priority) = priorities.last() {
            match priority {
                super::prophet::RemovalAssumption::Removed(loc) => {
                    self.unassumable_remove.insert(*loc);
                }
                super::prophet::RemovalAssumption::NotRemoved(loc) => {
                    self.unassumable_keep.insert(*loc);
                }
                super::prophet::RemovalAssumption::Move(from_loc, to_loc) => {
                    self.cant_move.insert((*from_loc, *to_loc));
                }
            }
        }
    }

    /// Convert SAT model to attack actions
    fn convert_model_to_actions(
        &self,
        model: &Model<'ctx>,
        paths: &[MovementSegment],
        cycles: &[MovementCycle],
        board: &Board,
    ) -> Result<Vec<AttackAction>> {
        let mut actions = Vec::new();
        let mut visited_segments = HashSet::new();

        // Sort segments by attack time
        let mut all_segments: Vec<Box<dyn std::any::Any>> = Vec::new();

        // Add paths
        for path in paths {
            all_segments.push(Box::new((path.clone(), false)));
        }

        // Add cycles
        for cycle in cycles {
            all_segments.push(Box::new((cycle.clone(), true)));
        }

        // Sort by attack time (simplified for now)
        all_segments.sort_by_key(|seg| {
            if let Some((path, _)) = seg.downcast_ref::<(MovementSegment, bool)>() {
                path.attack_time
            } else if let Some((cycle, _)) = seg.downcast_ref::<(MovementCycle, bool)>() {
                cycle.attack_time
            } else {
                0
            }
        });

        // Process segments in order
        for segment in all_segments {
            if let Some((path, is_cycle)) = segment.downcast_ref::<(MovementSegment, bool)>() {
                if *is_cycle {
                    // This shouldn't happen, but handle it gracefully
                    continue;
                }

                // Process path
                if !visited_segments.contains(&path.attacker_loc) {
                    // Add moves from dest_n end to attacker_loc end
                    for i in (1..path.destinations.len()).rev() {
                        actions.push(AttackAction::Move {
                            from_loc: path.destinations[i],
                            to_loc: path.destinations[i - 1],
                        });
                    }

                    // Add the attacker_loc's attack if defender not already killed
                    if self.graph.attackers.contains(&path.attacker_loc) {
                        // This would need to check the actual model state
                        // and add appropriate attacks
                    }

                    visited_segments.insert(path.attacker_loc);
                }
            } else if let Some((cycle, is_cycle)) = segment.downcast_ref::<(MovementCycle, bool)>()
            {
                if !*is_cycle {
                    // This shouldn't happen, but handle it gracefully
                    continue;
                }

                // Process cycle
                if !visited_segments.contains(&cycle.locs[0]) {
                    // Add cyclic move
                    actions.push(AttackAction::MoveCyclic {
                        locs: cycle.locs.clone(),
                    });

                    // Mark all segments in cycle as visited
                    for loc in &cycle.locs {
                        visited_segments.insert(*loc);
                    }

                    // Add attacks for all attackers in cycle
                    for loc in &cycle.locs {
                        if self.graph.attackers.contains(loc) {
                            // Add attack if defender not already killed
                            // This would need to check the actual model state
                        }
                    }
                }
            }
        }

        Ok(actions)
    }

    fn print_cost_matrix(&self, priorities: &[super::prophet::RemovalAssumption]) {
        let mut all_destinations = HashSet::new();
        for (_, hex_map) in &self.graph.move_hex_map {
            for (dest, _) in hex_map {
                all_destinations.insert(*dest);
            }
        }
        let friend_locs: Vec<Loc> = self.graph.friends.iter().cloned().collect();
        let destination_locs: Vec<Loc> = all_destinations.into_iter().collect();
        println!("[AttackSolver] Cost matrix (friend x dest):");
        for (friend_idx, &friend_loc) in friend_locs.iter().enumerate() {
            if let Some(hex_map) = self.graph.move_hex_map.get(&friend_loc) {
                for (dest_idx, &dest_loc) in destination_locs.iter().enumerate() {
                    let move_allowed = priorities.iter().any(|ass| match ass {
                        super::prophet::RemovalAssumption::Move(f, t) => {
                            *f == friend_loc && *t == dest_loc
                        }
                        _ => false,
                    });
                    let mut cost = f64::INFINITY;
                    if move_allowed {
                        if let Some(dnf) = hex_map.get(&dest_loc) {
                            let distance = friend_loc.dist(&dest_loc) as f64;
                            let move_prob = if dnf.is_none() { 1.0 } else { 0.5 };
                            if move_prob > 0.0 {
                                cost = -(move_prob as f64).log2() + distance * 0.1;
                            }
                        }
                    }
                    if cost < f64::INFINITY {
                        println!("  {:?} -> {:?}: cost {:.2}", friend_loc, dest_loc, cost);
                    }
                }
            }
        }
    }
}
