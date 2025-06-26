use crate::core::{  
    board::{Board, BitboardOps}, 
    loc::Loc, 
    side::Side, 
    tech::TechState, 
    units::Unit, 
    convert::FromIndex,
    board::actions::SpawnAction
};

/// Given the current board state and available money, this function decides which units to
/// purchase and where to spawn them using a set of heuristics.
pub fn generate_heuristic_spawn_actions(
    board: &Board,
    side: Side,
    tech_state: &TechState,
    mut money: i32,
) -> Vec<SpawnAction> {
    let mut actions = Vec::new();

    // Part 1: Decide what to buy and create `Buy` actions
    let units_to_buy = purchase_heuristic(side, tech_state, money);
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
                actions.push(SpawnAction::Spawn { spawn_loc: loc, unit });
                land_spawn_locs.set(loc, false);
            } else {
                // no more spawn locations
                break;
            }
        } else {
            // land units
            if let Some(loc) = land_spawn_locs.pop() {
                actions.push(SpawnAction::Spawn { spawn_loc: loc, unit });
                all_spawn_locs.set(loc, false);
            } 
        }
    }

    actions
}

/// Decides which units to buy based on available money and technology.
/// A simple greedy approach: keep buying the cheapest available unit.
fn purchase_heuristic(side: Side, tech_state: &TechState, mut money: i32) -> Vec<Unit> {
    let mut units_to_spawn = Vec::new();
    let mut available_units: Vec<_> = (0..Unit::NUM_UNITS)
        .map(|i| Unit::from_index(i).unwrap())
        .filter(|u| tech_state.can_buy(side, *u))
        .collect();
    available_units.sort_by_key(|u| u.stats().cost);

    if available_units.is_empty() {
        return units_to_spawn;
    }

    // Greedily buy the cheapest unit.
    let cheapest_unit = available_units[0];
    let cost = cheapest_unit.stats().cost;

    if cost <= 0 {
        return units_to_spawn;
    }

    while money >= cost {
        units_to_spawn.push(cheapest_unit);
        money -= cost;
    }
    units_to_spawn
}


#[cfg(test)]
mod tests {


    use super::*;
    use crate::core::board::{Board, Piece};
    use crate::core::map::Map;
    use crate::core::loc::Loc;
    use crate::core::side::Side;
    use crate::core::tech::{Tech, TechAssignment, TechState, Techline};
    use crate::core::units::Unit;

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
            .assign_techs(assignment_s0, Side::S0, &techline)
            .unwrap();
        // Don't unwrap, as some techs may have been acquired by S0 already
        let _ = tech_state.assign_techs(assignment_s1, Side::S1, &techline);

        tech_state
    }

    #[test]
    fn test_purchase_heuristic_basic() {
        let tech_state = new_all_unlocked_tech_state();
        let money = 100;
        let units = purchase_heuristic(Side::S0, &tech_state, money);
        // With 100 money, we should buy 50 Initiates (cost 2).
        assert_eq!(units.len(), 50);
        assert!(units.iter().all(|&u| u == Unit::Initiate));
    }

    #[test]
    fn test_purchase_heuristic_not_enough_money() {
        let tech_state = new_all_unlocked_tech_state();
        let money = 0;
        let units = purchase_heuristic(Side::S0, &tech_state, money);
        assert!(units.is_empty());
    }

    #[test]
    fn test_purchase_heuristic_exact_money() {
        let tech_state = new_all_unlocked_tech_state();
        let money = 1; // Not enough for an Initiate (cost 2)
        let units = purchase_heuristic(Side::S0, &tech_state, money);
        assert_eq!(units.len(), 0);
    }

    #[test]
    fn test_purchase_heuristic_respects_tech() {
        let mut tech_state = TechState::new();
        let techline = Techline::default();
        let initiate_tech = Tech::UnitTech(Unit::Initiate);
        let initiate_tech_index = techline
            .techs
            .iter()
            .position(|&t| t == initiate_tech)
            .unwrap();

        // Advance far enough to unlock Initiate, then acquire it.
        let assignment = TechAssignment::new(initiate_tech_index + 1, vec![initiate_tech_index]);
        tech_state
            .assign_techs(assignment, Side::S0, &techline)
            .unwrap();

        let money = 100;
        let units = purchase_heuristic(Side::S0, &tech_state, money);
        // Only Initiates are unlocked.
        assert_eq!(units.len(), 50);
        assert!(units.iter().all(|&u| u == Unit::Initiate));
    }

    #[test]
    fn test_get_spawn_locs_s0() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);
        // Add a spawner unit
        board.add_piece(Piece::new(Unit::BasicNecromancer, Side::S0, Loc::new(1, 1)));
        let locs = board.get_spawn_locs(Side::S0, true);
        // Necromancer at (1,1) can spawn at 6 adjacent hexes in the spawn zone.
        assert_eq!(locs.len(), 6);
        assert!(locs.iter().all(|loc| loc.y <= 2));
    }

    #[test]
    fn test_get_spawn_locs_s1() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);
        board.add_piece(Piece::new(Unit::BasicNecromancer, Side::S1, Loc::new(1, 8)));
        let locs = board.get_spawn_locs(Side::S1, true);
        assert_eq!(locs.len(), 6);
        assert!(locs.iter().all(|loc| loc.y >= 7));
    }

    #[test]
    fn test_get_spawn_locs_blocked() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);
        // Add a spawner
        board.add_piece(Piece::new(Unit::BasicNecromancer, Side::S0, Loc::new(1, 1)));

        // Block one of the spawn locations
        let blocked_loc = Loc::new(1, 0);
        board.add_piece(Piece::new(Unit::Zombie, Side::S0, blocked_loc));

        let locs = board.get_spawn_locs(Side::S0, true);
        assert_eq!(locs.len(), 5);
        assert!(!locs.contains(&blocked_loc));
    }

    #[test]
    fn test_generate_heuristic_spawn_actions_full() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);
        board.add_piece(Piece::new(Unit::BasicNecromancer, Side::S0, Loc::new(4, 1)));

        let tech_state = new_all_unlocked_tech_state();
        let mut money = 4;
        let actions = generate_heuristic_spawn_actions(&board, Side::S0, &tech_state, money);

        // It should generate 2 Buy actions and 2 Spawn actions.
        assert_eq!(actions.len(), 4);

        let mut buy_count = 0;
        let mut spawn_count = 0;
        for action in &actions {
            match action {
                SpawnAction::Buy { unit } => {
                    assert_eq!(*unit, Unit::Initiate);
                    buy_count += 1;
                }
                SpawnAction::Spawn { unit, .. } => {
                    assert_eq!(*unit, Unit::Initiate);
                    spawn_count += 1;
                }
                _ => panic!("Unexpected action type"),
            }
        }
        assert_eq!(buy_count, 2);
        assert_eq!(spawn_count, 2);
    }
}