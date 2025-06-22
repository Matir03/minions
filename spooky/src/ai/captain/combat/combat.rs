use std::collections::{HashMap, HashSet};

use crate::core::{Board, Loc, Side};
use crate::core::board::Piece;

impl Board {
    pub fn identify_combat_pairs(&self, side_to_move: Side) -> Vec<CombatPair> {
        let mut pairs = Vec::new();

        // Get all pieces by side
        let mut friendly_pieces = Vec::new();
        let mut enemy_pieces = Vec::new();

        for (loc, piece) in &self.pieces {
            if piece.side == side_to_move {
                friendly_pieces.push((*loc, piece));
            } else {
                enemy_pieces.push((*loc, piece));
            }
        }

        // For each friendly piece, find all possible targets
        for (attacker_pos, attacker) in &friendly_pieces {
            let attacker_stats = attacker.unit.stats();

            // Skip pieces that can't attack
            if attacker_stats.num_attacks == 0 {
                continue;
            }

            for (defender_pos, _defender) in &enemy_pieces {
                // Calculate all possible attack hexes for this pair
                let attack_hexes = self.get_attack_hexes(*attacker_pos, *defender_pos, attacker_stats);

                if !attack_hexes.is_empty() {
                    pairs.push(CombatPair {
                        attacker_pos: *attacker_pos,
                        defender_pos: *defender_pos,
                        attack_hexes,
                    });
                }
            }
        }

        pairs
    }

    fn get_attack_hexes(&self, attacker_pos: Loc, defender_pos: Loc, attacker_stats: &crate::core::units::UnitStats) -> Vec<Loc> {
        let mut attack_hexes = Vec::new();
        let valid_move_hexes = self.get_valid_move_hexes(attacker_pos);

        for candidate_pos in valid_move_hexes.iter().chain(std::iter::once(&attacker_pos)) {
            if candidate_pos.dist(&defender_pos) <= attacker_stats.range {
                attack_hexes.push(*candidate_pos);
            }
        }

        attack_hexes
    }

    pub fn combat_graph(&self, side: Side) -> CombatGraph {
        let pairs = self.identify_combat_pairs(side);
        let friendlies: Vec<Loc> = self.pieces
            .iter()
            .filter(|(_, p)| p.side == side)
            .map(|(loc, _)| *loc)
            .collect();
        CombatGraph::new(pairs, friendlies, self)
    }
}

#[derive(Debug, Clone)]
pub struct CombatPair {
    pub attacker_pos: Loc,
    pub defender_pos: Loc,
    pub attack_hexes: Vec<Loc>,
}

#[derive(Clone)]
pub struct CombatGraph {
    pub pairs: Vec<CombatPair>,
    pub friendlies: Vec<Loc>,
    pub defenders: Vec<Loc>,
    pub hexes: HashSet<Loc>,
    pub attack_hexes: HashMap<Loc, Vec<Loc>>,
    pub friendly_move_hexes: HashMap<Loc, Vec<Loc>>,
}

impl CombatGraph {
    pub fn new(pairs: Vec<CombatPair>, friendlies: Vec<Loc>, board: &Board) -> Self {
        let mut friendly_move_hexes = HashMap::new();
        for friendly_loc in &friendlies {
            friendly_move_hexes.insert(*friendly_loc, board.get_valid_move_hexes(*friendly_loc));
        }

        let mut defenders = HashSet::new();
        let mut hexes = HashSet::new();
        let mut attack_hexes: HashMap<Loc, Vec<Loc>> = HashMap::new();

        for pair in &pairs {
            defenders.insert(pair.defender_pos);
            for hex in &pair.attack_hexes {
                hexes.insert(*hex);
            }
            attack_hexes.entry(pair.attacker_pos).or_default().extend(pair.attack_hexes.clone());
        }

        Self {
            pairs,
            friendlies,
            defenders: defenders.into_iter().collect(),
            hexes,
            attack_hexes,
            friendly_move_hexes,
        }
    }
}