use std::collections::HashMap;
use crate::core::{Loc, Side, units::UnitStats};
use crate::core::board::Piece;

/// A prophecy represents predicted probabilities for pieces being passive or removed
#[derive(Debug, Clone)]
pub struct Prophecy {
    pub passive_probabilities: HashMap<Loc, f32>,
    pub removal_probabilities: HashMap<Loc, f32>,
}

impl Prophecy {
    pub fn new() -> Self {
        Self {
            passive_probabilities: HashMap::new(),
            removal_probabilities: HashMap::new(),
        }
    }

    /// Generate a basic prophecy using primitive heuristics
    pub fn generate_basic(board: &crate::core::Board, side: Side) -> Self {
        let mut prophecy = Prophecy::new();

        // Get all pieces by side
        let mut friendly_pieces = Vec::new();
        let mut enemy_pieces = Vec::new();

        for (loc, piece) in &board.pieces {
            if piece.side == side {
                friendly_pieces.push((*loc, piece));
            } else {
                enemy_pieces.push((*loc, piece));
            }
        }

        // Generate passive probabilities for friendly pieces
        for (loc, piece) in &friendly_pieces {
            let stats = piece.unit.stats();
            let mut passive_prob: f32 = 0.5; // Base probability

            // Adjust based on piece value and position
            if stats.cost > 4 {
                passive_prob -= 0.2; // Expensive pieces more likely to attack
            }

            // Adjust based on health
            let damage_taken = piece.state.damage_taken;
            let health_ratio = (stats.defense - damage_taken) as f32 / stats.defense as f32;
            if health_ratio < 0.5 {
                passive_prob += 0.3; // Damaged pieces more likely to be passive
            }

            // Adjust based on movement state
            if piece.state.moved && stats.lumbering {
                passive_prob += 0.4; // Lumbering pieces that moved are likely passive
            }

            prophecy.passive_probabilities.insert(*loc, passive_prob.max(0.0).min(1.0));
        }

        // Generate removal probabilities for enemy pieces
        for (loc, piece) in &enemy_pieces {
            let stats = piece.unit.stats();
            let mut removal_prob = 0.3; // Base probability

            // Adjust based on piece value
            if stats.cost > 4 {
                removal_prob += 0.2; // Expensive pieces more likely to be targeted
            }

            // Adjust based on health
            let damage_taken = piece.state.damage_taken;
            let health_ratio = (stats.defense - damage_taken) as f32 / stats.defense as f32;
            if health_ratio < 0.5 {
                removal_prob += 0.3; // Damaged pieces more likely to be removed
            }

            // Adjust based on piece type
            if stats.necromancer {
                removal_prob += 0.4; // Necromancers are high priority targets
            }

            // Adjust based on position (pieces near friendly pieces more likely to be attacked)
            let mut nearby_friendly = 0;
            for (friendly_loc, _) in &friendly_pieces {
                if loc.dist(friendly_loc) <= 2 {
                    nearby_friendly += 1;
                }
            }
            if nearby_friendly > 0 {
                removal_prob += 0.1 * nearby_friendly as f32;
            }

            prophecy.removal_probabilities.insert(*loc, removal_prob.max(0.0).min(1.0));
        }

        prophecy
    }

    /// Add random noise to create a variation of the prophecy
    pub fn add_noise(&self, noise_factor: f32) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let mut new_prophecy = self.clone();

        for prob in new_prophecy.passive_probabilities.values_mut() {
            let noise = rng.gen_range(-noise_factor..noise_factor);
            *prob = (*prob + noise).max(0.0).min(1.0);
        }

        for prob in new_prophecy.removal_probabilities.values_mut() {
            let noise = rng.gen_range(-noise_factor..noise_factor);
            *prob = (*prob + noise).max(0.0).min(1.0);
        }

        new_prophecy
    }
}