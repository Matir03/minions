use crate::ai::captain::board_node::BoardPreTurn;
use crate::ai::captain::combat::graph::CombatGraph;
use crate::ai::captain::positioning::MoveCandidate;
use crate::core::{board::actions::BoardTurn, Board, GameConfig, Loc, Side, Sigmoid};
use crate::heuristics::naive::{CombinedEnc, NaiveHeuristic};
use crate::heuristics::{BoardHeuristic, Heuristic, RemovalAssumption};
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

const DIST_EXP: f64 = 0.8;

fn loc_wt(spec: &crate::core::MapSpec, loc: &Loc) -> f64 {
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

    fn compute_board_turn(
        &self,
        blotto: i32,
        shared: &CombinedEnc<'a>,
        enc: &Self::BoardEnc,
    ) -> BoardTurn {
        BoardTurn::default()
    }

    fn compute_board_pre_turn(
        &self,
        rng: &mut impl Rng,
        shared: &CombinedEnc<'a>,
        enc: &Self::BoardEnc,
        board: &Board<'a>,
        side: Side,
    ) -> BoardPreTurn {
        let graph = board.combat_graph(side);
        let moves = generate_move_candidates(rng, &graph, board, side);
        let removals = generate_assumptions(board, side);

        BoardPreTurn { moves, removals }
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

    let unif_distr = rand::distributions::Uniform::new(-NOISE, NOISE);

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
                + rng.sample(unif_distr);

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
