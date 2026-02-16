use crate::ai::captain::board_node::BoardAttackPhasePreTurn;
use crate::ai::captain::combat::graph::CombatGraph;
use crate::ai::captain::positioning::MoveCandidate;
use crate::core::{
    board::{
        actions::{BoardTurn, SetupAction, SpawnAction},
        definitions::Phase,
        Bitboard, BitboardOps,
    },
    tech::{Tech, TechState},
    Board, GameConfig, Loc, MapSpec, Side, Sigmoid, ToIndex, Unit,
};
use crate::heuristics::{
    smart::{zones::ZoneAnalysis, CombinedEnc, SmartHeuristic},
    BoardHeuristic, BoardSetupPhasePreTurn, BoardSpawnPhasePreTurn, Heuristic, RemovalAssumption,
};
use rand::distr::{weighted::WeightedIndex, Distribution, Uniform};
use rand::prelude::*;
use rand::Rng;
use std::collections::{HashMap, HashSet};

const DIST_EXP: f64 = 0.8;

fn loc_wt(spec: &MapSpec, loc: &Loc) -> f64 {
    spec.graveyards
        .iter()
        .map(|g| DIST_EXP.powi(g.dist(loc)))
        .sum::<f64>()
}

/// Encoding stored per board for the smart heuristic.
/// Tracks how many times this board state has been revisited so that noise
/// can be scaled up on subsequent heuristic evaluations.
#[derive(Debug, Clone)]
pub struct SmartBoardEnc {
    pub revisit_count: u32,
}

impl<'a> BoardHeuristic<'a, CombinedEnc<'a>> for SmartHeuristic<'a> {
    type BoardEnc = SmartBoardEnc;

    fn compute_enc(&self, _board: &Board<'a>) -> Self::BoardEnc {
        SmartBoardEnc { revisit_count: 0 }
    }

    fn update_enc(&self, enc: &Self::BoardEnc, _turn: &BoardTurn) -> Self::BoardEnc {
        SmartBoardEnc {
            revisit_count: enc.revisit_count + 1,
        }
    }

    fn compute_board_attack_phase_pre_turn(
        &self,
        rng: &mut impl Rng,
        _shared: &CombinedEnc<'a>,
        enc: &Self::BoardEnc,
        board: &Board<'a>,
        side: Side,
    ) -> BoardAttackPhasePreTurn {
        let graph = board.combat_graph(side);
        let zone_analysis = ZoneAnalysis::compute(board);
        let moves =
            generate_move_candidates(rng, &graph, board, side, enc.revisit_count, &zone_analysis);
        let removals = generate_attack_strategy(rng, board, side);

        BoardAttackPhasePreTurn { moves, removals }
    }

    fn compute_board_setup_phase_pre_turn(
        &self,
        _rng: &mut impl Rng,
        _shared: &CombinedEnc<'a>,
        _enc: &Self::BoardEnc,
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
        _shared: &CombinedEnc<'a>,
        _enc: &Self::BoardEnc,
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
    revisit_count: u32,
    zone_analysis: &ZoneAnalysis,
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
    const BASE_NOISE: f64 = 0.05;

    // Noise scales with revisit count: more exploration on later visits.
    let noise_scale = BASE_NOISE * (1.0 + 0.2 * revisit_count as f64);
    let unif_distr = Uniform::new(-noise_scale, noise_scale).unwrap();

    // Enemy attack reach for follow-up move penalties.
    let enemy_attack = zone_analysis.attack_reach[(!side) as usize];
    let friendly_attack = zone_analysis.attack_reach[side as usize];

    for (from_loc, to_loc_map) in graph.move_hex_map.iter() {
        let unit_stats = board.get_piece(&from_loc).unwrap().unit.stats();
        if chosen_pieces.contains_key(&from_loc) {
            let g = chosen_pieces[&from_loc];
            let g_dist = g.dist(from_loc);
            for to_loc in to_loc_map.keys() {
                let delta_dist = (g_dist - to_loc.dist(&g)) as f64;
                let mut score = if delta_dist > 0.0 {
                    1.0 - (3.0 - delta_dist) * EPS
                } else {
                    (3.0 + delta_dist) * EPS
                };

                // Follow-up penalty: penalize destinations attackable by enemy
                // without friendly counter-attack.
                if enemy_attack.get(*to_loc) && !friendly_attack.get(*to_loc) {
                    score -= 0.15;
                } else if !enemy_attack.get(*to_loc) {
                    score += 0.05;
                }

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
            let mut score = ((to_wt - cur_wt) * unit_stats.cost as f64 / SCORE_SIGMOID_SCALE)
                .sigmoid()
                + unif_distr.sample(rng);

            // Follow-up penalty for non-chosen pieces too.
            if enemy_attack.get(*to_loc) && !friendly_attack.get(*to_loc) {
                score -= 0.10;
            } else if !enemy_attack.get(*to_loc) {
                score += 0.03;
            }

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

/// Choose an attack strategy randomly and produce removal assumptions.
///
/// Four strategies:
///   1. Kill-everything-by-value (sorted by cost descending)
///   2. Kill-by-type-locality (kill all of a single unit type within radius 3)
///   3. Kill-nothing (Keep all defenders)
///   4. Kill-random-subset (random fraction of defenders)
pub fn generate_attack_strategy(
    rng: &mut impl Rng,
    board: &Board,
    side: Side,
) -> Vec<RemovalAssumption> {
    let graph = board.combat_graph(side);
    let defenders: Vec<Loc> = graph.defenders.iter().copied().collect();

    if defenders.is_empty() {
        return Vec::new();
    }

    // Weighted selection among strategies: 40% kill-by-value, 20% kill-by-type,
    // 15% kill-nothing, 25% kill-random-subset.
    let strategy_weights = [40, 20, 15, 25];
    let dist = WeightedIndex::new(&strategy_weights).unwrap();
    let strategy = dist.sample(rng);

    match strategy {
        0 => strategy_kill_by_value(board, &defenders),
        1 => strategy_kill_by_type_locality(rng, board, &defenders),
        2 => strategy_kill_nothing(&defenders),
        3 => strategy_kill_random_subset(rng, board, &defenders),
        _ => unreachable!(),
    }
}

/// Strategy 0: Kill everything sorted by value (necromancers first,
/// then by net cost descending). This is the original behavior.
fn strategy_kill_by_value(board: &Board, defenders: &[Loc]) -> Vec<RemovalAssumption> {
    let mut scored: Vec<(Loc, f64)> = defenders
        .iter()
        .map(|loc| {
            let piece = board.get_piece(loc).unwrap();
            let stats = piece.unit.stats();
            let score = if stats.necromancer {
                1.0
            } else {
                (stats.cost - stats.rebate) as f64 / 10.0
            };
            (*loc, score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    scored
        .into_iter()
        .flat_map(|(loc, score)| {
            vec![
                (RemovalAssumption::Kill(loc), score),
                (RemovalAssumption::Unsummon(loc), 0.01),
                (RemovalAssumption::Keep(loc), -1.0),
            ]
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(a, _s)| {
            // already sorted within each defender block by kill > unsummon > keep
            a
        })
        .collect()
}

/// Strategy 1: For each unit type present among defenders, kill all of that
/// type within radius 3 of a random defender of that type. Keep the rest.
fn strategy_kill_by_type_locality(
    rng: &mut impl Rng,
    board: &Board,
    defenders: &[Loc],
) -> Vec<RemovalAssumption> {
    // Group defenders by unit type.
    let mut by_type: HashMap<Unit, Vec<Loc>> = HashMap::new();
    for loc in defenders {
        let piece = board.get_piece(loc).unwrap();
        by_type.entry(piece.unit).or_default().push(*loc);
    }

    // Pick a random unit type.
    let types: Vec<Unit> = by_type.keys().copied().collect();
    let chosen_type = types[rng.random_range(0..types.len())];
    let type_locs = &by_type[&chosen_type];

    // Pick a random anchor among defenders of that type.
    let anchor = type_locs[rng.random_range(0..type_locs.len())];

    // Kill all of that type within distance 3 of the anchor.
    let kill_set: HashSet<Loc> = type_locs
        .iter()
        .filter(|l| anchor.dist(l) <= 3)
        .copied()
        .collect();

    defenders
        .iter()
        .map(|loc| {
            if kill_set.contains(loc) {
                RemovalAssumption::Kill(*loc)
            } else {
                RemovalAssumption::Keep(*loc)
            }
        })
        .collect()
}

/// Strategy 2: Keep all defenders (explore purely positional moves).
fn strategy_kill_nothing(defenders: &[Loc]) -> Vec<RemovalAssumption> {
    defenders
        .iter()
        .map(|loc| RemovalAssumption::Keep(*loc))
        .collect()
}

/// Strategy 3: Kill a random fraction of defenders. The fraction itself is
/// drawn uniformly from [0, 1].
fn strategy_kill_random_subset(
    rng: &mut impl Rng,
    board: &Board,
    defenders: &[Loc],
) -> Vec<RemovalAssumption> {
    let fraction: f64 = rng.random();

    // Score defenders by value, take the top `fraction` portion.
    let mut scored: Vec<(Loc, f64)> = defenders
        .iter()
        .map(|loc| {
            let piece = board.get_piece(loc).unwrap();
            let stats = piece.unit.stats();
            let score = if stats.necromancer {
                100.0
            } else {
                (stats.cost - stats.rebate) as f64
            };
            (*loc, score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let kill_count = (defenders.len() as f64 * fraction).ceil() as usize;
    let kill_set: HashSet<Loc> = scored.iter().take(kill_count).map(|(l, _)| *l).collect();

    defenders
        .iter()
        .map(|loc| {
            if kill_set.contains(loc) {
                RemovalAssumption::Kill(*loc)
            } else {
                RemovalAssumption::Keep(*loc)
            }
        })
        .collect()
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

    // Compute zone analysis for strategic spawn location selection.
    let zone_analysis = ZoneAnalysis::compute(board);
    let enemy_attack = zone_analysis.attack_reach[(!side) as usize];
    let graveyard_bb = board.bitboards.graveyards;

    // Get spawn locations based on the current board state.
    let mut all_spawn_locs = board.bitboards.get_spawn_locs(side, true);
    let mut land_spawn_locs = board.bitboards.get_spawn_locs(side, false);

    /// Score a spawn location for strategic value.
    fn score_spawn_loc(
        loc: Loc,
        graveyard_bb: Bitboard,
        enemy_attack: Bitboard,
        zone_analysis: &ZoneAnalysis,
        side: Side,
        map_spec: &MapSpec,
    ) -> f64 {
        let mut score = 0.0;

        // Graveyard proximity (higher if near unoccupied graveyards).
        let graveyard_locs = graveyard_bb.to_locs();
        for g in &graveyard_locs {
            let dist = loc.dist(g);
            if dist <= 3 {
                let proximity = 1.0 / (1.0 + dist as f64);
                score += proximity * 3.0;
            }
        }

        // On a graveyard itself: big bonus.
        if graveyard_bb.get(loc) {
            score += 5.0;
        }

        // Bonus for being in a covered/protected zone for our side.
        use crate::heuristics::smart::zones::HexZone;
        match zone_analysis.zone_at(loc) {
            HexZone::Protected(s) if s == side => score += 2.0,
            HexZone::Covered(s) if s == side => score += 1.0,
            HexZone::Contested => score += 0.2,
            HexZone::Open => {}
            HexZone::Covered(s) if s != side => score -= 0.5,
            HexZone::Protected(s) if s != side => score -= 1.0,
            _ => {}
        }

        // Penalty if enemy can attack this hex.
        if enemy_attack.get(loc) {
            score -= 2.0;
        }

        // Small weight from the existing map-based weighting.
        score += loc_wt(map_spec, &loc) * 0.5;

        score
    }

    for unit in sorted_units {
        let spawn_bb = if unit.stats().flying {
            all_spawn_locs
        } else {
            land_spawn_locs
        };

        if spawn_bb == 0 {
            break;
        }

        // Score each available spawn location and pick the best.
        let mut best_loc: Option<Loc> = None;
        let mut best_score = f64::NEG_INFINITY;

        let mut scan = spawn_bb;
        while let Some(candidate) = scan.pop() {
            let s = score_spawn_loc(
                candidate,
                graveyard_bb,
                enemy_attack,
                &zone_analysis,
                side,
                board.map.spec(),
            );
            if s > best_score {
                best_score = s;
                best_loc = Some(candidate);
            }
        }

        if let Some(loc) = best_loc {
            actions.push(SpawnAction::Spawn {
                spawn_loc: loc,
                unit,
            });
            all_spawn_locs.set(loc, false);
            land_spawn_locs.set(loc, false);
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
