use std::collections::{HashMap, HashSet};

use crate::core::{Board, Loc, Side};
use crate::core::board::Piece;

#[derive(Debug, Clone)]
pub struct CombatPair {
    pub attacker_pos: Loc,
    pub defender_pos: Loc,
    pub attack_hexes: Vec<Loc>,
}

impl Board {
    pub fn get_valid_move_hexes(&self, piece_loc: Loc) -> Vec<Loc> {
        let mut valid_hexes = Vec::new();
        if let Some(piece) = self.get_piece(&piece_loc) {
            let stats = piece.unit.stats();
            for y in 0..10 {
                for x in 0..10 {
                    let target_loc = Loc::new(x, y);
                    if piece_loc.dist(&target_loc) <= stats.speed {
                        // Can't move to a hex occupied by another unit.
                        if !self.pieces.contains_key(&target_loc) || target_loc == piece_loc {
                            valid_hexes.push(target_loc);
                        }
                    }
                }
            }
        }
        valid_hexes
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

        // If attacker is lumbering and has moved, they can only attack from their current position
        let can_move = !attacker_stats.lumbering || !self.pieces[&attacker_pos].state.borrow().moved;

        if can_move {
            // Check all positions within movement range
            for y in 0..10 {
                for x in 0..10 {
                    let candidate_pos = Loc::new(x, y);

                    // Check if this position is within movement range
                    if attacker_pos.dist(&candidate_pos) <= attacker_stats.speed {
                        // Check if this position can attack the defender
                        if candidate_pos.dist(&defender_pos) <= attacker_stats.range {
                            // Check if the path is blocked (simplified - just check if position is occupied by enemy)
                            let is_blocked = self.pieces.get(&candidate_pos)
                                .map(|piece| piece.side != Side::S0)
                                .unwrap_or(false);

                            if !is_blocked {
                                attack_hexes.push(candidate_pos);
                            }
                        }
                    }
                }
            }
        } else {
            // Can only attack from current position
            if attacker_pos.dist(&defender_pos) <= attacker_stats.range {
                attack_hexes.push(attacker_pos);
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