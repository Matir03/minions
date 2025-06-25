use std::collections::{HashMap, HashSet};

use crate::core::loc::PATH_MAPS;
use crate::core::map::Terrain;
use crate::core::{Board, Loc, Side};
use crate::core::board::Piece;

impl<'a> Board<'a> {
    pub fn identify_combat_triples(&self, side_to_move: Side) -> Vec<CombatTriple> {
        let mut triples = Vec::new();

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
            let valid_move_hexes = self.get_valid_move_hexes(*attacker_pos);

            for (defender_pos, _defender) in &enemy_pieces {
                // Calculate all possible attack hexes for this pair
                let attack_hexes = self.get_valid_move_hexes(*attacker_pos)
                    .into_iter()
                    .filter(|hex| hex.dist(defender_pos) <= attacker_stats.range)
                    .collect::<Vec<Loc>>();

                if !attack_hexes.is_empty() {
                    triples.push(CombatTriple {
                        attacker_pos: *attacker_pos,
                        defender_pos: *defender_pos,
                        attack_hexes,
                    });
                }
            }
        }

        triples
    }

    pub fn combat_graph(&self, side: Side) -> CombatGraph {
        let triples = self.identify_combat_triples(side);
        let friendlies: Vec<Loc> = self.pieces
            .iter()
            .filter(|(_, p)| p.side == side)
            .map(|(loc, _)| *loc)
            .collect();
        CombatGraph::new(triples, friendlies, self)
    }
}

#[derive(Debug, Clone)]
pub struct CombatTriple {
    pub attacker_pos: Loc,
    pub defender_pos: Loc,
    pub attack_hexes: Vec<Loc>,
}

// A DNF formula representing the removal of enemy units
// that allows a friendly unit to move to a given hex
// inner vec is AND, outer vec is OR
// None means it no removal is needed
pub type RemovalDNF = Option<Vec<Vec<Loc>>>;

#[derive(Debug, Clone)]
pub struct CombatGraph {
    // attacker-defender-location triples
    pub triples: Vec<CombatTriple>,

    // all friendly units
    pub friends: Vec<Loc>,
    // enemy units that can be attacked
    pub defenders: Vec<Loc>,

    // all hexes that could be moved to by each friendly unit
    // if sufficient enemy units are removed
    pub move_hex_map: HashMap<Loc, 
        HashMap<Loc, RemovalDNF>
    >,

    // list of defenders for given attacker
    pub attacker_to_defenders_map: HashMap<Loc, Vec<Loc>>,
    // list of attackers for given defender
    pub defender_to_attackers_map: HashMap<Loc, Vec<Loc>>,
    
}

impl CombatGraph {
    pub fn new(triples: Vec<CombatTriple>, friends: Vec<Loc>, board: &Board) -> Self {
        let mut move_hex_map = HashMap::new();

        let mut defenders = HashSet::new();
        let mut attacker_to_defenders_map = HashMap::new();
        let mut defender_to_attackers_map = HashMap::new();

        for CombatTriple {attacker_pos, defender_pos, ..} in &triples {
            defenders.insert(*defender_pos);
            attacker_to_defenders_map.entry(*attacker_pos).or_insert_with(Vec::new).push(*defender_pos);
            defender_to_attackers_map.entry(*defender_pos).or_insert_with(Vec::new).push(*attacker_pos);
        }

        for friend in &friends {
            let piece = board.get_piece(friend).unwrap();
            let flying = piece.unit.stats().flying;

            let free_move_hexes = board.get_unobstructed_move_hexes(*friend)
                .into_iter()
                .collect::<HashSet<Loc>>();

            let theoretical_move_hexes = board.get_theoretical_move_hexes(*friend);
            let dnfs = theoretical_move_hexes.iter().map(|hex| {
                if free_move_hexes.contains(hex) {
                    return (*hex, None);
                }

                if flying {
                    return (*hex, Some(vec![vec![*hex]]));
                }

                let paths = friend.paths_to(hex);
                
                let dnf = paths.into_iter().filter_map(|path| {
                    let mut conjunct = vec![];

                    for loc in path {
                        if board.map.get_terrain(&loc) == Some(Terrain::Flood) {
                            return None;
                        }

                        if defenders.contains(&loc) {
                            conjunct.push(loc);
                        }
                    }

                    Some(conjunct)
                }).collect::<Vec<Vec<Loc>>>();

                (*hex, Some(dnf))
            }).collect::<HashMap<Loc, RemovalDNF>>();

            move_hex_map.insert(*friend, dnfs);
        }

        Self {
            triples,
            friends,
            defenders: defenders.into_iter().collect(),
            move_hex_map,
            attacker_to_defenders_map,
            defender_to_attackers_map,
        }
    }
}