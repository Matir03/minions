use crate::core::board::Piece;
use crate::core::{units::UnitStats, Loc, Side};
use rand::prelude::*;
use std::collections::HashMap;

/// Represents an assumption about whether a unit should be removed
#[derive(Debug, Clone)]
pub enum RemovalAssumption {
    Removed(Loc),
    NotRemoved(Loc),
}

impl RemovalAssumption {
    pub fn loc(&self) -> Loc {
        match self {
            RemovalAssumption::Removed(loc) => *loc,
            RemovalAssumption::NotRemoved(loc) => *loc,
        }
    }
}

/// Death Prophet that generates removal assumptions in order of priority
pub struct DeathProphet {
    rng: StdRng,
}

impl DeathProphet {
    pub fn new(rng: StdRng) -> Self {
        Self { rng }
    }

    /// Generate a list of removal assumptions ordered by priority
    /// For now, this generates random assumptions as specified in the architecture
    pub fn generate_assumptions(
        &mut self,
        board: &crate::core::Board,
        side: Side,
    ) -> Vec<RemovalAssumption> {
        let mut assumptions = Vec::new();
        let mut priority_scores = HashMap::new();

        // Get the combat graph to see which defenders are actually attackable
        let graph = board.combat_graph(side);
        let attackable_defenders: std::collections::HashSet<_> =
            graph.defenders.iter().cloned().collect();

        // Get all enemy pieces (potential defenders) that are actually attackable
        for (loc, piece) in &board.pieces {
            if piece.side != side && attackable_defenders.contains(loc) {
                // Generate a random number between -1 and 1 for each defender
                let score = self.rng.gen_range(-1.0..1.0);
                priority_scores.insert(*loc, score);
            }
        }

        // Convert scores to assumptions and sort by absolute value
        let mut scored_assumptions: Vec<_> = priority_scores
            .into_iter()
            .map(|(loc, score): (Loc, f64)| {
                let assumption = if score > 0.0 {
                    RemovalAssumption::Removed(loc)
                } else {
                    RemovalAssumption::NotRemoved(loc)
                };
                (assumption, score.abs())
            })
            .collect();

        // Sort by absolute value of score (highest priority first)
        scored_assumptions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Extract just the assumptions in priority order
        assumptions = scored_assumptions
            .into_iter()
            .map(|(assumption, _)| assumption)
            .collect();

        assumptions
    }

    /// Get feedback on the maximum prefix of assumptions that can be satisfied
    /// This is called by the combat generation system
    pub fn receive_feedback(&mut self, _max_satisfiable_prefix: usize) {
        // For now, we don't use feedback to adjust future assumptions
        // In a more sophisticated implementation, this could be used to learn
        // which types of assumptions are more likely to be satisfiable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::rng::make_rng;
    use crate::core::{board::Board, map::Map, side::Side, units::Unit};

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
