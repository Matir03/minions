use crate::ai::captain::board_node::BoardAttackPhasePreTurn;
use crate::ai::captain::combat::graph::CombatGraph;
use crate::ai::captain::positioning::MoveCandidate;
use crate::core::{
    board::{
        actions::{BoardTurn, SetupAction, SpawnAction},
        definitions::Phase,
        BitboardOps,
    },
    tech::{Tech, TechState},
    Board, GameConfig, Loc, MapSpec, Side, Sigmoid, ToIndex, Unit,
};
use crate::heuristics::{
    naive::{CombinedEnc, NaiveHeuristic},
    BoardHeuristic, BoardSetupPhasePreTurn, BoardSpawnPhasePreTurn, Heuristic, RemovalAssumption,
};
use rand::distr::{weighted::WeightedIndex, Distribution};
use rand::prelude::*;
use std::collections::{HashMap, HashSet};

const DIST_EXP: f64 = 0.8;

fn loc_wt(spec: &MapSpec, loc: &Loc) -> f64 {
    spec.graveyards
        .iter()
        .map(|g| DIST_EXP.powi(g.dist(loc)))
        .sum::<f64>()
}

impl<'a> BoardHeuristic<'a, CombinedEnc<'a>> for NaiveHeuristic<'a> {
    type BoardEnc = ();

    fn compute_enc(&self, board: &Board<'a>) -> Self::BoardEnc {
        ()
    }

    fn update_enc(&self, enc: &Self::BoardEnc, turn: &BoardTurn) -> Self::BoardEnc {
        ()
    }

    fn compute_board_attack_phase_pre_turn(
        &self,
        rng: &mut impl Rng,
        shared: &CombinedEnc<'a>,
        enc: &Self::BoardEnc,
        board: &Board<'a>,
        side: Side,
    ) -> BoardAttackPhasePreTurn {
        let graph = board.combat_graph(side);
        let moves = generate_move_candidates(rng, &graph, board, side);
        let removals = generate_assumptions(board, side);

        BoardAttackPhasePreTurn { moves, removals }
    }

    fn compute_board_setup_phase_pre_turn(
        &self,
        rng: &mut impl Rng,
        shared: &CombinedEnc<'a>,
        enc: &Self::BoardEnc,
        board: &Board<'a>,
        side: Side,
    ) -> BoardSetupPhasePreTurn {
        if !board.state.phases().contains(&Phase::Setup) {
            return None;
        }

        let saved_unit = board.reinforcements[side]
            .iter()
            .max_by_key(|unit| (unit.stats().cost, unit.to_index().unwrap()))
            .cloned();

        Some(SetupAction {
            necromancer_choice: Unit::BasicNecromancer,
            saved_unit,
        })
    }

    fn compute_board_spawn_phase_pre_turn(
        &self,
        rng: &mut impl Rng,
        shared: &CombinedEnc<'a>,
        enc: &Self::BoardEnc,
        board: &Board<'a>,
        side: Side,
        money: i32,
        tech_state: &TechState,
    ) -> BoardSpawnPhasePreTurn {
        generate_spawn_actions(board, side, tech_state, money, rng)
    }
}

pub fn generate_move_candidates(
    rng: &mut impl Rng,
    graph: &CombatGraph,
    board: &Board,
    side: Side,
) -> Vec<MoveCandidate> {
    let mut candidates: Vec<MoveCandidate> = Vec::new();

    let graveyards = &board.map.spec().graveyards;
    let sidedness = graveyards
        .iter()
        .map(|g| {
            let mut total_diff = 0.0;
            let mut total_wt = 0.0;

            for (loc, piece) in &board.pieces {
                let dist = g.dist(loc);
                let dist_wt = DIST_EXP.powi(dist);
                let wt = piece.unit.stats().cost as f64 * dist_wt;
                let sd = (side.sign() * piece.side.sign()) as f64;

                total_diff += sd * wt;
                total_wt += wt;
            }

            let sidedness = total_diff / total_wt;
            (g, sidedness)
        })
        .collect::<Vec<_>>();

    // graveyard -> (piece, turns to graveyard)
    let turns_to_graveyard = graveyards
        .iter()
        .map(|g| {
            let mut turns_to_graveyard = board
                .pieces
                .iter()
                .filter(|(_, piece)| piece.side == side)
                .map(|(loc, piece)| {
                    let dist = g.dist(loc);
                    let speed = piece.unit.stats().speed;
                    let turns = if speed == 0 {
                        f64::INFINITY
                    } else {
                        dist as f64 / speed as f64
                    };
                    (*loc, turns)
                })
                .collect::<Vec<_>>();

            turns_to_graveyard.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            (*g, turns_to_graveyard)
        })
        .collect::<HashMap<_, _>>();

    let mut chosen_pieces: HashMap<Loc, Loc> = HashMap::new();

    let mut sidedness_decreasing = sidedness.clone();
    sidedness_decreasing.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let mut competitiveness_increasing = sidedness.clone();
    competitiveness_increasing.sort_by(|(_, a), (_, b)| a.abs().partial_cmp(&b.abs()).unwrap());

    for (g, s) in sidedness_decreasing {
        if s < 0.0 {
            break;
        }

        let best_piece = turns_to_graveyard[&g]
            .iter()
            .find(|(piece_loc, _)| !chosen_pieces.contains_key(piece_loc));

        if let Some((piece_loc, _)) = best_piece {
            chosen_pieces.insert(*piece_loc, *g);
        } else {
            break;
        }
    }

    for (g, _) in competitiveness_increasing {
        let best_piece = turns_to_graveyard[&g].iter().find(|(piece_loc, _)| {
            board.get_piece(piece_loc).unwrap().unit.stats().spawn
                && !chosen_pieces.contains_key(piece_loc)
        });

        if let Some((piece_loc, _)) = best_piece {
            chosen_pieces.insert(*piece_loc, *g);
        } else {
            break;
        }
    }

    const EPS: f64 = 1e-6;
    const NOISE: f64 = 0.05;

    let unif_distr = rand::distr::Uniform::new(-NOISE, NOISE).unwrap();

    for (from_loc, to_loc_map) in graph.move_hex_map.iter() {
        let unit_stats = board.get_piece(&from_loc).unwrap().unit.stats();
        if chosen_pieces.contains_key(&from_loc) {
            let g = chosen_pieces[&from_loc];
            let g_dist = g.dist(from_loc);

            for to_loc in to_loc_map.keys() {
                let delta_dist = (g_dist - to_loc.dist(&g)) as f64;
                let score = if delta_dist > 0.0 {
                    1.0 - (3.0 - delta_dist) * EPS
                } else {
                    (3.0 + delta_dist) * EPS
                };

                candidates.push(MoveCandidate::Move {
                    from_loc: *from_loc,
                    to_loc: *to_loc,
                    score,
                });

                if unit_stats.blink {
                    candidates.push(MoveCandidate::Blink {
                        loc: *from_loc,
                        score: 0.0,
                    });
                }
            }

            continue;
        }

        let cur_wt = loc_wt(board.map.spec(), from_loc);

        for (to_loc, _) in to_loc_map {
            let to_wt = loc_wt(board.map.spec(), to_loc);
            const SCORE_SIGMOID_SCALE: f64 = 10.0;
            let score = ((to_wt - cur_wt) * unit_stats.cost as f64 / SCORE_SIGMOID_SCALE).sigmoid()
                + unif_distr.sample(rng);

            candidates.push(MoveCandidate::Move {
                from_loc: *from_loc,
                to_loc: *to_loc,
                score,
            });

            if unit_stats.blink {
                candidates.push(MoveCandidate::Blink {
                    loc: *from_loc,
                    score: 0.5,
                    // TODO: make this a parameter
                });
            }
        }
    }

    // Sort candidates by score in descending order
    candidates.sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap());

    candidates
}

pub fn generate_assumptions(board: &Board, side: Side) -> Vec<RemovalAssumption> {
    let mut scores = HashMap::new();

    // Get the combat graph to see which defenders are actually attackable
    let graph = board.combat_graph(side);
    let defenders = &graph.defenders;

    // Get all enemy pieces (potential defenders) that are actually attackable
    for loc in defenders {
        let piece = board.get_piece(loc).unwrap();
        let piece_stats = piece.unit.stats();
        let score = if piece_stats.necromancer {
            1.0
        } else {
            (piece_stats.cost - piece_stats.rebate) as f64 / 10.0
        };
        scores.insert(*loc, score);
    }

    // Convert scores to assumptions and sort
    let mut scored_assumptions: Vec<_> = scores
        .into_iter()
        .flat_map(|(loc, score): (Loc, f64)| {
            vec![
                (RemovalAssumption::Kill(loc), score),
                (RemovalAssumption::Unsummon(loc), 0.01),
                (RemovalAssumption::Keep(loc), -1.0),
            ]
        })
        .collect();

    // Sort by scores
    scored_assumptions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Extract just the assumptions in priority order
    let assumptions = scored_assumptions
        .into_iter()
        .map(|(assumption, _)| assumption)
        .collect();

    assumptions
}

const WEIGHT_FACTOR: f64 = 1.2;

/// Given the current board state and available money, this function decides which units to
/// purchase and where to spawn them using a set of heuristics.
pub fn generate_spawn_actions(
    board: &Board,
    side: Side,
    tech_state: &TechState,
    mut money: i32,
    rng: &mut impl Rng,
) -> Vec<SpawnAction> {
    let mut actions = Vec::new();

    // Part 1: Decide what to buy and create `Buy` actions
    let units_to_buy = purchase_heuristic(side, tech_state, money, rng);
    for &unit in &units_to_buy {
        money -= unit.stats().cost;
        actions.push(SpawnAction::Buy { unit });
    }

    // Part 2: Decide what to spawn from all available reinforcements (original + newly bought)
    let mut all_units_to_potentially_spawn = board.reinforcements[side].clone();
    for &unit in &units_to_buy {
        all_units_to_potentially_spawn.insert(unit);
    }

    // Greedily spawn the most expensive units.
    let mut sorted_units = all_units_to_potentially_spawn
        .iter()
        .copied()
        .collect::<Vec<_>>();
    sorted_units.sort_by_key(|u| -(u.stats().cost as i32));

    // Get spawn locations based on the current board state.
    let mut all_spawn_locs = board.bitboards.get_spawn_locs(side, true);
    let mut land_spawn_locs = board.bitboards.get_spawn_locs(side, false);

    for unit in sorted_units {
        if unit.stats().flying {
            if let Some(loc) = all_spawn_locs.pop() {
                actions.push(SpawnAction::Spawn {
                    spawn_loc: loc,
                    unit,
                });
                land_spawn_locs.set(loc, false);
            } else {
                // no more spawn locations
                break;
            }
        } else {
            // land units
            if let Some(loc) = land_spawn_locs.pop() {
                actions.push(SpawnAction::Spawn {
                    spawn_loc: loc,
                    unit,
                });
                all_spawn_locs.set(loc, false);
            }
        }
    }

    actions
}

/// Decides which units to buy based on available money and tech
/// weight units by their tech index, buy later units at a higher rate
fn purchase_heuristic(
    side: Side,
    tech_state: &TechState,
    mut money: i32,
    rng: &mut impl Rng,
) -> Vec<Unit> {
    let mut available_units: Vec<_> = tech_state.acquired_techs[side]
        .iter()
        .filter_map(|tech| {
            if let Tech::UnitTech(unit) = tech {
                Some(*unit)
            } else {
                None
            }
        })
        .chain(Unit::BASIC_UNITS.into_iter())
        .collect();

    let mut units_with_weights = available_units
        .iter()
        .map(|u| (u, WEIGHT_FACTOR.powi(u.to_index().unwrap() as i32)))
        .collect::<Vec<_>>();

    let mut units_to_buy = Vec::new();

    loop {
        units_with_weights = units_with_weights
            .into_iter()
            .filter(|(u, _)| money >= u.stats().cost)
            .collect();

        if units_with_weights.is_empty() {
            break;
        }

        let weights: Vec<f64> = units_with_weights.iter().map(|(_, w)| *w).collect();
        let distr = WeightedIndex::new(&weights).unwrap();
        let idx = distr.sample(rng);
        let unit = units_with_weights[idx].0;

        units_to_buy.push(*unit);
        money -= unit.stats().cost;
    }

    units_to_buy
}
