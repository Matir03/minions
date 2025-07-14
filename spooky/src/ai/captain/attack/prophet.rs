use crate::core::{board::Board, units::UnitStats, Loc, Move, Side};
use rand::prelude::*;
use std::collections::{HashMap, HashSet};

/// Represents an assumption about whether a unit should be removed or moved
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttackConstraint {
    Remove(Loc),
    Keep(Loc),
}

impl AttackConstraint {
    pub fn loc(&self) -> Loc {
        match self {
            AttackConstraint::Remove(loc) => *loc,
            AttackConstraint::Keep(loc) => *loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveAssumption {
    pub mv: Move,
    pub cost: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttackAssumption {
    pub constraint: AttackConstraint,
    pub cost: f64,
}

#[derive(Debug, Clone)]
pub struct Prophecy {
    pub move_assumptions: Vec<MoveAssumption>,
    pub attack_assumptions: Vec<AttackAssumption>,
}

/// Death Prophet that generates removal assumptions in order of priority
pub struct DeathProphet {
    rng: StdRng,
    last_prophecy: Option<Prophecy>,
}

pub fn score_to_cost(score: f64) -> f64 {
    -score.log2()
}

impl DeathProphet {
    pub fn new(rng: StdRng) -> Self {
        Self {
            rng,
            last_prophecy: None,
        }
    }

    /// Generate a list of removal and move assumptions ordered by priority
    pub fn make_prophecy(&mut self, board: &Board, side: Side) -> Prophecy {
        let mut move_priorities = Vec::new();
        let mut attack_priorities = Vec::new();

        // Get the combat graph to see which defenders are actually attackable
        let graph = board.combat_graph(side);
        let defenders = &graph.defenders;

        // Get all enemy pieces (potential defenders) that are actually attackable
        for loc in defenders {
            let piece = board.get_piece(loc).unwrap();
            let piece_stats = piece.unit.stats();
            let removal_score = if piece_stats.necromancer {
                1.0
            } else {
                (piece_stats.cost - piece_stats.rebate) as f64 / 10.0
            };

            attack_priorities.push(AttackAssumption {
                constraint: AttackConstraint::Remove(*loc),
                cost: score_to_cost(removal_score),
            });
            attack_priorities.push(AttackAssumption {
                constraint: AttackConstraint::Keep(*loc),
                cost: score_to_cost(1.0 - removal_score),
            });
        }
        attack_priorities.sort_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap());

        // Add move assumptions from move_hex_map
        for (friend_loc, hex_map) in &graph.move_hex_map {
            for (dest_loc, dnf) in hex_map {
                // Weight moves by distance from starting location
                let distance = friend_loc.dist(dest_loc) as f64;
                let cost = 3.0 - distance;

                move_priorities.push(MoveAssumption {
                    mv: Move {
                        from: *friend_loc,
                        to: *dest_loc,
                    },
                    cost,
                });
            }
        }
        move_priorities.sort_by(|a, b| a.cost.partial_cmp(&b.cost).unwrap());

        Prophecy {
            move_assumptions: move_priorities,
            attack_assumptions: attack_priorities,
        }
    }

    /// Get feedback on the maximum prefix of assumptions that can be satisfied
    /// This is called by the combat generation system
    pub fn receive_feedback(&mut self, active_constraints: Vec<bool>) {
        let satisfied_assumptions = self
            .last_prophecy
            .iter()
            .zip(active_constraints)
            .filter(|(_, active)| *active)
            .map(|(assumption, _)| assumption)
            .collect::<Vec<_>>();

        println!("Satisfiable assumptions: {:?}", satisfied_assumptions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::rng::make_rng;
    use crate::core::{
        board::{Board, Piece},
        map::Map,
        side::Side,
        units::Unit,
    };

    #[test]
    fn test_death_prophet_generates_assumptions() {
        let map = Map::BlackenedShores;
        let mut board = Board::new(&map);

        // Place both enemy pieces adjacent to the friendly piece
        let loc1 = crate::core::loc::Loc::new(5, 5);
        let loc2 = crate::core::loc::Loc::new(4, 6);
        board.add_piece(Piece::new(Unit::Zombie, Side::Blue, loc1));
        board.add_piece(Piece::new(Unit::Initiate, Side::Blue, loc2));

        // Add a friendly piece that can attack the enemy pieces
        board.add_piece(Piece::new(
            Unit::Zombie,
            Side::Yellow,
            crate::core::loc::Loc::new(4, 5),
        ));

        let graph = board.combat_graph(Side::Yellow);
        println!("Defenders in combat graph: {:?}", graph.defenders);
        println!("Enemy pieces on board: {:?}", vec![loc1, loc2]);

        let mut prophet = DeathProphet::new(make_rng());
        let prophecy = prophet.make_prophecy(&board, Side::Yellow);

        // Should generate removal assumptions for both enemy pieces since they're attackable
        let removal_assumptions: Vec<_> = prophecy.attack_assumptions;
        assert_eq!(removal_assumptions.len(), 4);
        // For each enemy piece, check that both Removed and NotRemoved exist
        for &loc in &[loc1, loc2] {
            assert!(removal_assumptions
                .iter()
                .any(|a| matches!(a.constraint, AttackConstraint::Remove(l) if l == loc)));
            assert!(removal_assumptions
                .iter()
                .any(|a| matches!(a.constraint, AttackConstraint::Keep(l) if l == loc)));
        }
    }

    #[test]
    fn test_assumptions_are_prioritized() {
        let map = Map::BlackenedShores;
        let board = Board::new(&map);

        let mut prophet = DeathProphet::new(make_rng());
        let prophecy1 = prophet.make_prophecy(&board, Side::Yellow);
        let prophecy2 = prophet.make_prophecy(&board, Side::Yellow);

        // Even with no pieces, we should get consistent behavior (empty lists)
        assert_eq!(
            prophecy1.attack_assumptions.len(),
            prophecy2.attack_assumptions.len()
        );
    }
}
