use crate::core::{
    action::BoardAction,
    board::Board,
    convert::FromIndex,
    loc::Loc,
    side::Side,
    tech::TechState,
    units::{Unit, NUM_UNITS},
};

/// Given the current board state and available money, this function decides which units to
/// purchase and where to spawn them using a set of heuristics.
pub fn generate_heuristic_spawn_actions(
    board: &Board,
    side: Side,
    tech_state: &TechState,
    money: i32,
) -> Vec<BoardAction> {
    let units_to_spawn = purchase_heuristic(side, tech_state, money);
    let mut available_spawn_locs = get_spawn_locs(board, side);

    // Simple placement heuristic: sort units by cost (most expensive first)
    // and place them one by one in the first available spawn location.
    let mut actions = Vec::new();
    for unit in units_to_spawn {
        if let Some(loc) = available_spawn_locs.pop() {
            actions.push(BoardAction::Spawn { spawn_loc: loc, unit });
        } else {
            // No more available spawn locations
            break;
        }
    }
    actions
}

/// Decides which units to buy based on available money and technology.
/// A simple greedy approach: keep buying the cheapest available unit.
fn purchase_heuristic(side: Side, tech_state: &TechState, mut money: i32) -> Vec<Unit> {
    let mut units_to_spawn = Vec::new();
    let mut available_units: Vec<_> = (0..NUM_UNITS)
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

/// Returns a list of valid, empty locations where the given side can spawn units.
fn get_spawn_locs(board: &Board, side: Side) -> Vec<Loc> {
    let y_range = if side == Side::S0 { 0..=2 } else { 7..=9 };
    let mut locs = Vec::new();
    for y in y_range {
        for x in 0..=9 {
            let loc = Loc::new(x, y);
            if !board.pieces.contains_key(&loc) {
                locs.push(loc);
            }
        }
    }
    // Sort by distance from center, then y, then x to be deterministic.
    // The most central locations will be at the end of the vector, to be popped.
    locs.sort_by(|a, b| {
        let dist_sq_a = (2 * a.x - 9).pow(2) + (2 * a.y - 9).pow(2);
        let dist_sq_b = (2 * b.x - 9).pow(2) + (2 * b.y - 9).pow(2);
        dist_sq_b
            .cmp(&dist_sq_a)
            .then_with(|| {
                if side == Side::S0 {
                    a.y.cmp(&b.y)
                } else {
                    b.y.cmp(&a.y)
                }
            })
            .then_with(|| a.x.cmp(&b.x))
    });
    locs
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
        let board = Board::new(Map::BlackenedShores);
        let locs = get_spawn_locs(&board, Side::S0);
        assert_eq!(locs.len(), 30); // 3 rows of 10
        assert!(locs.iter().all(|loc| loc.y <= 2));
    }

    #[test]
    fn test_get_spawn_locs_s1() {
        let board = Board::new(Map::BlackenedShores);
        let locs = get_spawn_locs(&board, Side::S1);
        assert_eq!(locs.len(), 30); // 3 rows of 10
        assert!(locs.iter().all(|loc| loc.y >= 7));
    }

    #[test]
    fn test_get_spawn_locs_blocked() {
        let mut board = Board::new(Map::BlackenedShores);
        let loc = Loc::new(0, 0);
        let piece = Piece {
            loc,
            side: Side::S0,
            unit: Unit::Initiate,
            modifiers: Default::default(),
            state: Default::default(),
        };
        board.add_piece(piece);
        let locs = get_spawn_locs(&board, Side::S0);
        assert_eq!(locs.len(), 29);
        assert!(!locs.contains(&loc));
    }

    #[test]
    fn test_generate_heuristic_spawn_actions_full() {
        let board = Board::new(Map::BlackenedShores);
        let tech_state = new_all_unlocked_tech_state();
        // With 4 money, it should buy 2 Initiates (cost 2 each)
        let money = 4;
        let actions = generate_heuristic_spawn_actions(&board, Side::S0, &tech_state, money);
        assert_eq!(actions.len(), 2);

        // All spawned units should be Initiates
        for action in &actions {
            if let BoardAction::Spawn { unit, .. } = action {
                assert_eq!(*unit, Unit::Initiate);
            } else {
                panic!("Expected a spawn action");
            }
        }

        // Test placement order. The first action is for the first unit from the purchase
        // heuristic (all Initiates, so order doesn't matter), and it should be placed
        // in the most central location. Because the code `pop()`s from the spawn list,
        // this corresponds to the last element of the sorted `get_spawn_locs` vector.
        if let BoardAction::Spawn { spawn_loc, .. } = &actions[0] {
            assert_eq!(*spawn_loc, Loc::new(5, 2));
        } else {
            panic!("Expected a spawn action");
        }
    }

    #[test]
    fn test_get_spawn_locs_is_deterministic() {
        let board = Board::new(Map::BlackenedShores);
        let locs_s0 = get_spawn_locs(&board, Side::S0);
        // The most central locations should be at the end.
        // The two most central are (4, 2) and (5, 2).
        // With our deterministic sort (dist DESC, then y ASC, then x ASC), (5, 2) should be last.
        assert_eq!(*locs_s0.last().unwrap(), Loc::new(5, 2));
        assert_eq!(locs_s0[locs_s0.len() - 2], Loc::new(4, 2));

        let locs_s1 = get_spawn_locs(&board, Side::S1);
        // For S1, most central are (4, 7) and (5, 7).
        // With our deterministic sort (dist DESC, then y DESC, then x ASC), (5, 7) should be last.
        assert_eq!(*locs_s1.last().unwrap(), Loc::new(5, 7));
        assert_eq!(locs_s1[locs_s1.len() - 2], Loc::new(4, 7));
    }
}