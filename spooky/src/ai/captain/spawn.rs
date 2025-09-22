use std::collections::{HashMap, HashSet};

use crate::core::{
    board::{actions::SpawnAction, BitboardOps, Board},
    convert::FromIndex,
    loc::Loc,
    side::Side,
    tech::TechState,
    units::Unit,
    Tech, ToIndex,
};
use rand::{distributions::WeightedIndex, prelude::*};

/// Given the current board state and available money, this function decides which units to
/// purchase and where to spawn them using a set of heuristics.
pub fn generate_heuristic_spawn_actions(
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

const WEIGHT_FACTOR: f64 = 1.2;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::board::{Board, Piece};
    use crate::core::loc::Loc;
    use crate::core::map::Map;
    use crate::core::side::Side;
    use crate::core::tech::{Tech, TechAssignment, TechState, Techline};
    use crate::core::units::Unit;
    use crate::utils::make_rng;

    fn new_all_unlocked_tech_state() -> TechState {
        let mut tech_state = TechState::new();
        let techline = Techline::default();
        let mut acquire_indices = Vec::new();

        // Collect all 0-based indices for unit techs
        for i in 0..techline.len() {
            if let Tech::UnitTech(unit) = techline[i] {
                if unit.stats().cost > 0 {
                    acquire_indices.push(i);
                }
            }
        }

        // To acquire techs, we must first advance the unlock_index past them.
        let assignment_s0 = TechAssignment::new(techline.len(), acquire_indices.clone());
        let assignment_s1 = TechAssignment::new(techline.len(), acquire_indices);

        tech_state
            .assign_techs(assignment_s0, Side::Yellow, &techline)
            .unwrap();
        // Don't unwrap, as some techs may have been acquired by S0 already
        let _ = tech_state.assign_techs(assignment_s1, Side::Blue, &techline);

        tech_state
    }

    #[test]
    fn test_purchase_heuristic_not_enough_money() {
        let tech_state = new_all_unlocked_tech_state();
        let money = 0;
        let board = Board::new(&Map::BlackenedShores);
        let units = purchase_heuristic(Side::Yellow, &tech_state, money, &mut make_rng());
        assert!(units.is_empty());
    }

    #[test]
    fn test_purchase_heuristic_exact_money() {
        let tech_state = new_all_unlocked_tech_state();
        let money = 1; // Not enough money for any unit
        let units = purchase_heuristic(Side::Yellow, &tech_state, money, &mut make_rng());
        assert!(units.is_empty());
    }

    #[test]
    fn test_get_spawn_locs_s0() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);
        // Add a spawner unit
        board.add_piece(Piece::new(
            Unit::BasicNecromancer,
            Side::Yellow,
            Loc::new(1, 1),
        ));
        let locs = board.get_spawn_locs(Side::Yellow, true);
        // Necromancer at (1,1) can spawn at 6 adjacent hexes in the spawn zone.
        assert_eq!(locs.len(), 6);
        assert!(locs.iter().all(|loc| loc.y <= 2));
    }

    #[test]
    fn test_get_spawn_locs_s1() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);
        board.add_piece(Piece::new(
            Unit::BasicNecromancer,
            Side::Blue,
            Loc::new(1, 8),
        ));
        let locs = board.get_spawn_locs(Side::Blue, true);
        assert_eq!(locs.len(), 6);
        assert!(locs.iter().all(|loc| loc.y >= 7));
    }

    #[test]
    fn test_get_spawn_locs_blocked() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);
        // Add a spawner
        board.add_piece(Piece::new(
            Unit::BasicNecromancer,
            Side::Yellow,
            Loc::new(1, 1),
        ));

        // Block one of the spawn locations
        let blocked_loc = Loc::new(1, 0);
        board.add_piece(Piece::new(Unit::Zombie, Side::Yellow, blocked_loc));

        let locs = board.get_spawn_locs(Side::Yellow, true);
        assert_eq!(locs.len(), 5);
        assert!(!locs.contains(&blocked_loc));
    }

    #[test]
    fn test_generate_heuristic_spawn_actions_full() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);
        board.add_piece(Piece::new(
            Unit::BasicNecromancer,
            Side::Yellow,
            Loc::new(4, 1),
        ));

        let tech_state = new_all_unlocked_tech_state();
        let mut money = 4;
        let actions = generate_heuristic_spawn_actions(
            &board,
            Side::Yellow,
            &tech_state,
            money,
            &mut make_rng(),
        );

        // It should generate 1 Buy action and 1 Spawn action.
        assert!(actions.len() >= 2);

        let mut buy_count = 0;
        let mut spawn_count = 0;
        for action in &actions {
            match action {
                SpawnAction::Buy { unit } => {
                    buy_count += 1;
                }
                SpawnAction::Spawn { unit, .. } => {
                    spawn_count += 1;
                }
                _ => panic!("Unexpected action type"),
            }
        }
        assert!(buy_count > 0);
        assert!(spawn_count > 0);
    }
}
