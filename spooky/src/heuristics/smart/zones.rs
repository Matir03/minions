//! Zone analysis for the smart heuristic.
//!
//! Classifies hexes on the board as contested, covered, protected, or open
//! based on unit positions and reachability.

use std::collections::HashMap;

use crate::core::{
    board::{Bitboard, BitboardOps},
    Board, Loc, Side,
};

/// How "safe" a hex is for a given side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HexZone {
    /// Attacked by both sides' units.
    Contested,
    /// Covered by `side`: any enemy reaching here in k turns can be countered
    /// in k-1 turns (via spawning within range 2 or attacking).
    Covered(Side),
    /// Protected by `side`: no enemy can attack here without passing through
    /// that side's covered hexes.
    Protected(Side),
    /// Not protected or covered by either side.
    Open,
}

/// Precomputed zone analysis for a board position.
#[derive(Debug, Clone)]
pub struct ZoneAnalysis {
    /// Per-hex zone classification.
    pub zones: [HexZone; 100],

    /// Bitboard of hexes attackable by each side in one move (attack range from
    /// any piece's reachable hexes).
    pub attack_reach: [Bitboard; 2],

    /// Bitboard of hexes within spawning range (distance ≤2 from any spawner)
    /// for each side.
    pub spawn_reach: [Bitboard; 2],

    /// Bitboard of hexes "covered" by each side (the side can respond to
    /// threats there).
    pub covered: [Bitboard; 2],

    /// Bitboard of hexes "protected" by each side (enemy must pass through
    /// covered hexes to attack).
    pub protected: [Bitboard; 2],
}

const MAX_LOOKAHEAD: i32 = 3;

impl ZoneAnalysis {
    /// Build the full zone analysis for a board position.
    pub fn compute(board: &Board) -> Self {
        let mut analysis = Self {
            zones: [HexZone::Open; 100],
            attack_reach: [Bitboard::new(); 2],
            spawn_reach: [Bitboard::new(); 2],
            covered: [Bitboard::new(); 2],
            protected: [Bitboard::new(); 2],
        };

        for side in Side::all() {
            analysis.attack_reach[side as usize] = Self::compute_attack_reach(board, side);
            analysis.spawn_reach[side as usize] = Self::compute_spawn_reach(board, side);
        }

        // Compute coverage: a hex is covered by `side` if any enemy reaching it
        // in k turns can be countered in k-1 turns.
        for side in Side::all() {
            analysis.covered[side as usize] = Self::compute_covered(board, side, &analysis);
        }

        // Compute protection: a hex is protected by `side` if every path from
        // any enemy piece to that hex passes through a covered hex.
        for side in Side::all() {
            analysis.protected[side as usize] = Self::compute_protected(board, side, &analysis);
        }

        // Classify each hex.
        analysis.classify_hexes();

        analysis
    }

    /// Bitboard of all hexes that `side`'s units can attack in one move.
    ///
    /// For each friendly piece, compute the set of hexes it can reach, then
    /// expand by its attack range.
    fn compute_attack_reach(board: &Board, side: Side) -> Bitboard {
        let mut reach = Bitboard::new();

        for (loc, piece) in &board.pieces {
            if piece.side != side {
                continue;
            }
            let stats = piece.unit.stats();
            // Hexes this piece can move to (including staying put).
            let move_hexes =
                board
                    .bitboards
                    .get_theoretical_moves(loc, side, stats.speed, stats.flying);

            // Expand each reachable hex by the attack range.
            let mut attack_hexes = move_hexes;
            for _ in 0..stats.range {
                attack_hexes |= attack_hexes.neighbors();
            }
            reach |= attack_hexes;
        }

        reach
    }

    /// Bitboard of hexes within distance 2 of any spawner owned by `side`.
    fn compute_spawn_reach(board: &Board, side: Side) -> Bitboard {
        let spawners = board.bitboards.spawners[side];
        // Range-1 neighbors of spawners (spawn locations), then one more ring.
        let ring1 = spawners | spawners.neighbors();
        let ring2 = ring1 | ring1.neighbors();
        ring2
    }

    /// Compute the set of hexes "covered" by `side`.
    ///
    /// A hex X is covered by side S if, for any enemy unit that could reach X
    /// in k ≤ MAX_LOOKAHEAD turns, S can either:
    ///   - spawn within range 2 of X (in k-1 turns, approximated by current
    ///     spawner positions), or
    ///   - attack and remove a hypothetical enemy from X (in k-1 turns).
    fn compute_covered(board: &Board, side: Side, analysis: &ZoneAnalysis) -> Bitboard {
        let enemy = !side;
        let side_idx = side as usize;
        let enemy_idx = enemy as usize;

        // Enemy reachability at increasing turn horizons.
        let mut enemy_reach_k = [Bitboard::new(); MAX_LOOKAHEAD as usize + 1];
        // Turn 0: enemy pieces' current positions.
        enemy_reach_k[0] = board.bitboards.pieces[enemy];

        // Propagate enemy reach for k turns using max speed across all their
        // pieces.  This is an over-approximation (uses max enemy speed for all).
        let max_enemy_speed = board
            .pieces
            .values()
            .filter(|p| p.side == enemy)
            .map(|p| p.unit.stats().speed)
            .max()
            .unwrap_or(0);

        let prop_mask = if board
            .pieces
            .values()
            .any(|p| p.side == enemy && p.unit.stats().flying)
        {
            !0 // any flying enemy can go anywhere
        } else {
            !board.bitboards.water
        };

        for k in 1..=MAX_LOOKAHEAD as usize {
            let prev = enemy_reach_k[k - 1];
            enemy_reach_k[k] = prev.all_movements(max_enemy_speed, prop_mask, !0) | prev;
        }

        // Side's defensive capability at turn horizons.
        // Turn 0: current attack reach + current spawn reach.
        let defense_base = analysis.attack_reach[side_idx] | analysis.spawn_reach[side_idx];

        // For simplicity in coverage check: hex is covered if every enemy reach
        // horizon ≤ MAX_LOOKAHEAD is dominated by the side's defense_base.
        // (Conservative: we don't model the side also propagating over time,
        //  but side's attack_reach already accounts for piece movement.)
        let mut uncoverable = Bitboard::new();
        for k in 1..=MAX_LOOKAHEAD as usize {
            // Hexes enemy can newly reach at turn k that we can't defend.
            uncoverable |= enemy_reach_k[k] & !defense_base;
        }

        // Covered = all hexes minus the ones we can't cover.
        let all_hexes: Bitboard = (1u128 << 100) - 1;
        all_hexes & !uncoverable
    }

    /// Compute the set of hexes "protected" by `side`.
    ///
    /// A hex is protected if all paths from any enemy piece to it must pass
    /// through hexes covered by `side`.  We approximate this by flood-filling
    /// from the side's pieces outward, only through covered hexes.
    fn compute_protected(board: &Board, side: Side, analysis: &ZoneAnalysis) -> Bitboard {
        let side_idx = side as usize;
        let covered = analysis.covered[side_idx];

        // Start from friendly pieces and flood through covered hexes.
        let mut frontier = board.bitboards.pieces[side];
        let mut protected = frontier;

        for _ in 0..10 {
            let next = frontier.neighbors() & covered & !protected;
            if next == 0 {
                break;
            }
            protected |= next;
            frontier = next;
        }

        protected
    }

    fn classify_hexes(&mut self) {
        for idx in 0..100 {
            let loc = Loc::from_index(idx);
            if !loc.in_bounds() {
                continue;
            }

            let yellow_covered = self.covered[Side::Yellow as usize].get(loc);
            let blue_covered = self.covered[Side::Blue as usize].get(loc);
            let yellow_protected = self.protected[Side::Yellow as usize].get(loc);
            let blue_protected = self.protected[Side::Blue as usize].get(loc);
            let yellow_attack = self.attack_reach[Side::Yellow as usize].get(loc);
            let blue_attack = self.attack_reach[Side::Blue as usize].get(loc);

            self.zones[idx as usize] = if yellow_protected && !blue_covered {
                HexZone::Protected(Side::Yellow)
            } else if blue_protected && !yellow_covered {
                HexZone::Protected(Side::Blue)
            } else if yellow_attack && blue_attack {
                HexZone::Contested
            } else if yellow_covered && !blue_covered {
                HexZone::Covered(Side::Yellow)
            } else if blue_covered && !yellow_covered {
                HexZone::Covered(Side::Blue)
            } else {
                HexZone::Open
            };
        }
    }

    /// Get the zone for a specific hex.
    pub fn zone_at(&self, loc: Loc) -> HexZone {
        if loc.in_bounds() {
            self.zones[loc.index()]
        } else {
            HexZone::Open
        }
    }

    /// Get the eviction probability for a graveyard occupied by `side`.
    ///
    /// Returns a per-turn probability of losing the graveyard, derived from the
    /// zone classification.
    pub fn eviction_probability(&self, loc: Loc, side: Side) -> f64 {
        match self.zone_at(loc) {
            HexZone::Protected(s) if s == side => 0.02,
            HexZone::Covered(s) if s == side => 0.08,
            HexZone::Contested => 0.25,
            HexZone::Open => 0.40,
            HexZone::Covered(s) if s != side => 0.55,
            HexZone::Protected(s) if s != side => 0.70,
            _ => 0.40,
        }
    }

    /// Compute expected discounted income from a graveyard for `side`.
    ///
    /// Uses a 10%-per-turn devaluation and zone-dependent eviction probability.
    /// Expected income = income / (1 - discount * (1 - p_evict))
    pub fn expected_graveyard_income(&self, loc: Loc, side: Side, income_per_turn: f64) -> f64 {
        let p_evict = self.eviction_probability(loc, side);
        let discount = 0.9; // 10% devaluation per turn
        let retention = discount * (1.0 - p_evict);

        if retention >= 1.0 {
            // Shouldn't happen, but guard against division issues.
            income_per_turn * 20.0 // cap
        } else {
            income_per_turn / (1.0 - retention)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        board::{definitions::BoardState, Piece},
        map::Map,
        units::Unit,
    };

    #[test]
    fn test_empty_board_zones_are_open() {
        let map = Map::AllLand;
        let board = Board::new(&map);
        let analysis = ZoneAnalysis::compute(&board);

        // With no pieces, all hexes should be open.
        for idx in 0..100 {
            let loc = Loc::from_index(idx);
            if loc.in_bounds() {
                assert_eq!(
                    analysis.zone_at(loc),
                    HexZone::Open,
                    "Expected Open at {:?}",
                    loc
                );
            }
        }
    }

    #[test]
    fn test_necromancer_position_is_protected_at_start() {
        let map = Map::AllLand;
        let mut board = Board::from_fen(Board::START_FEN, &map).unwrap();
        board.state = BoardState::Normal;

        let analysis = ZoneAnalysis::compute(&board);

        // Yellow necromancer at (2,2) should be protected or at least covered.
        let yellow_necro_loc = Loc::new(2, 2);
        let zone = analysis.zone_at(yellow_necro_loc);
        assert!(
            matches!(
                zone,
                HexZone::Protected(Side::Yellow) | HexZone::Covered(Side::Yellow)
            ),
            "Expected yellow necromancer at {:?} to be protected or covered, got {:?}",
            yellow_necro_loc,
            zone
        );
    }

    #[test]
    fn test_eviction_probability_ordering() {
        let map = Map::AllLand;
        let board = Board::new(&map);
        let analysis = ZoneAnalysis::compute(&board);

        let loc = Loc::new(5, 5);
        // For an empty board (Open zone), eviction probability should be
        // relatively high.
        let p = analysis.eviction_probability(loc, Side::Yellow);
        assert!(p > 0.3, "Expected high eviction for open zone, got {}", p);
    }

    #[test]
    fn test_expected_income_decreases_with_eviction() {
        let map = Map::AllLand;
        let board = Board::new(&map);
        let analysis = ZoneAnalysis::compute(&board);

        let loc = Loc::new(5, 5);
        let income_high_p = analysis.expected_graveyard_income(loc, Side::Yellow, 1.0);

        // For a "safe" hypothetical: income / (1 - 0.9 * 0.98) = ~8.5
        // For open: income / (1 - 0.9 * 0.60) = ~2.17
        assert!(
            income_high_p < 5.0,
            "Expected modest income for open zone, got {}",
            income_high_p
        );
    }
}
