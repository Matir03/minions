use std::collections::HashMap;

use crate::core::{Board, Loc, Side};
use crate::core::board::Piece;

#[derive(Debug, Clone)]
pub struct CombatPair {
    pub attacker_pos: Loc,
    pub defender_pos: Loc,
    pub attack_hexes: Vec<Loc>,
}

impl Board {
    pub fn identify_combat_pairs(&self) -> Vec<CombatPair> {
        let mut pairs = Vec::new();

        // Get all pieces by side
        let mut friendly_pieces = Vec::new();
        let mut enemy_pieces = Vec::new();

        for (loc, piece) in &self.pieces {
            if piece.side == Side::S0 {
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

    pub fn combat_graph(&self) -> CombatGraph {
        let pairs = self.identify_combat_pairs();
        CombatGraph::new(pairs)
    }
}

#[derive(Clone)]
pub struct CombatGraph {
    pub pairs: Vec<CombatPair>,
    pub hexes: Vec<Loc>,
    pub attackers: Vec<Loc>,
    pub attack_hexes: HashMap<Loc, Vec<Loc>>,
    pub hex_attackers: HashMap<Loc, Vec<Loc>>,
    pub defenders: Vec<Loc>,
}

impl CombatGraph {
    pub fn new(pairs: Vec<CombatPair>) -> Self {
        let mut hexes = Vec::new();
        let mut attackers = Vec::new();
        let mut defenders = Vec::new();
        let mut attack_hexes: HashMap<Loc, Vec<Loc>> = HashMap::new();
        let mut hex_attackers: HashMap<Loc, Vec<Loc>> = HashMap::new();

        // Collect all unique positions
        for pair in &pairs {
            if !attackers.contains(&pair.attacker_pos) {
                attackers.push(pair.attacker_pos);
            }
            if !defenders.contains(&pair.defender_pos) {
                defenders.push(pair.defender_pos);
            }

            for hex in &pair.attack_hexes {
                if !hexes.contains(hex) {
                    hexes.push(*hex);
                }

                // Build attack_hexes mapping (attacker -> attack hexes)
                attack_hexes.entry(pair.attacker_pos)
                    .or_insert_with(Vec::new)
                    .push(*hex);

                // Build hex_attackers mapping (hex -> attackers)
                hex_attackers.entry(*hex)
                    .or_insert_with(Vec::new)
                    .push(pair.attacker_pos);
            }
        }

        Self {
            pairs,
            hexes,
            attackers,
            attack_hexes,
            hex_attackers,
            defenders,
        }
    }
}