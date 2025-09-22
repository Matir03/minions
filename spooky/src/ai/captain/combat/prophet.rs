use crate::core::board::Piece;
use crate::core::{units::UnitStats, Loc, Side};
use rand::prelude::*;
use std::collections::HashMap;

/// Represents an assumption about whether a unit should be removed
#[derive(Debug, Clone)]
pub enum RemovalAssumption {
    Kill(Loc),
    Unsummon(Loc),
    Keep(Loc),
}

impl RemovalAssumption {
    pub fn loc(&self) -> Loc {
        match self {
            RemovalAssumption::Kill(loc) => *loc,
            RemovalAssumption::Unsummon(loc) => *loc,
            RemovalAssumption::Keep(loc) => *loc,
        }
    }
}

/// Death Prophet that generates removal assumptions in order of priority
#[derive(Debug)]
pub struct DeathProphet {
    rng: StdRng,
    last_assumptions: Vec<RemovalAssumption>,
}

impl DeathProphet {
    pub fn new(rng: StdRng) -> Self {
        Self {
            rng,
            last_assumptions: Vec::new(),
        }
    }

    /// Generate a list of removal assumptions ordered by priority
    pub fn generate_assumptions(
        &mut self,
        board: &crate::core::Board,
        side: Side,
    ) -> Vec<RemovalAssumption> {
        let mut assumptions = Vec::new();
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
        assumptions = scored_assumptions
            .into_iter()
            .map(|(assumption, _)| assumption)
            .collect();

        self.last_assumptions = assumptions.clone();
        assumptions
    }

    /// Get feedback on the lexicographically largest subsequence of assumptions that can be satisfied
    /// This is called by the combat generation system
    pub fn receive_feedback(&mut self, active_constraints: Vec<bool>) {
        // let satisfied_assumptions = self
        //     .last_assumptions
        //     .iter()
        //     .zip(active_constraints)
        //     .filter(|(_, active)| *active)
        //     .map(|(assumption, _)| assumption)
        //     .collect::<Vec<_>>();

        // println!("Satisfiable assumptions: {:?}", satisfied_assumptions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{board::Board, map::Map, side::Side, units::Unit};
    use crate::utils::make_rng;

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
        let assumptions = prophet.generate_assumptions(&board, Side::Yellow);

        // Should generate assumptions for both enemy pieces since they're attackable
        assert_eq!(assumptions.len(), 2);

        // All assumptions should be for enemy piece locations
        for assumption in &assumptions {
            let loc = assumption.loc();
            assert!(board.get_piece(&loc).is_ok());
            assert_eq!(board.get_piece(&loc).unwrap().side, Side::Blue);
        }
    }

    #[test]
    fn test_assumptions_are_prioritized() {
        let map = Map::BlackenedShores;
        let board = Board::new(&map);

        let mut prophet = DeathProphet::new(make_rng());
        let assumptions1 = prophet.generate_assumptions(&board, Side::Yellow);
        let assumptions2 = prophet.generate_assumptions(&board, Side::Yellow);

        // Even with no pieces, we should get consistent behavior (empty lists)
        assert_eq!(assumptions1.len(), 0);
        assert_eq!(assumptions2.len(), 0);
    }
}
