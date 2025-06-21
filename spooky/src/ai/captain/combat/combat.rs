use std::collections::{HashMap, HashSet};

use crate::core::{bitboards::{Bitboard, BitboardOps}, Board, Loc, Side};
use crate::core::board::Piece;

#[derive(Debug, Clone)]
pub struct CombatPair {
    pub attacker_pos: Loc,
    pub defender_pos: Loc,
    pub attack_hexes: Vec<Loc>,
}

impl Board {
    pub fn get_valid_move_hexes(&self, piece_loc: Loc) -> Vec<Loc> {
        if let Some(piece) = self.get_piece(&piece_loc) {
            let stats = piece.unit.stats();

            // Start with all hexes being potentially passable
            let mut passable_hexes = u128::MAX;

            // Flying units ignore all terrain
            if !stats.flying {
                // Ground units are blocked by certain terrain based on their stats
                passable_hexes &= !self.bitboards.water; // All non-flying are blocked by water
                if !stats.persistent {
                    passable_hexes &= !self.bitboards.whirlwind;
                }
                if stats.defense < 4 {
                    passable_hexes &= !self.bitboards.firestorm;
                }
                if stats.speed < 2 {
                    passable_hexes &= !self.bitboards.earthquake;
                }
            }

            let mut start_pos_bb = Bitboard::new();
            start_pos_bb.set(piece_loc, true);

            // Units can't move through hexes occupied by other pieces, but can move from their own hex.
            let prop_mask = (passable_hexes & !self.bitboards.all_pieces) | start_pos_bb;
            // Units can land on any unoccupied hex they can reach, or their start hex.
            let dest_mask = prop_mask;

            let reachable_hexes = start_pos_bb.all_movements(stats.speed, prop_mask, dest_mask);

            // Convert bitboard to Vec<Loc>
            let mut valid_hexes = Vec::new();
            for y in 0..10 {
                for x in 0..10 {
                    let loc = Loc::new(x, y);
                    if reachable_hexes.get(loc) {
                        valid_hexes.push(loc);
                    }
                }
            }
            return valid_hexes;
        }
        Vec::new()
    }

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

            for (defender_pos, defender) in &enemy_pieces {
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

        for candidate_pos in valid_move_hexes {
            if candidate_pos.dist(&defender_pos) <= attacker_stats.range {
                attack_hexes.push(candidate_pos);
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