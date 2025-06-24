//! Board representation and rules

pub mod definitions;
pub mod fen;
pub mod actions;
pub mod bitboards;
pub use bitboards::{Bitboard, BitboardOps, Bitboards};

pub use definitions::{Board, BoardState, Modifiers, Piece, PieceState};

use std::{cell::RefCell, collections::HashMap};
use anyhow::{anyhow, bail, ensure, Context, Result};
use self::actions::{SetupAction, AttackAction, SpawnAction};
use hashbag::HashBag;
use crate::core::{loc::GRID_LEN, side};

use super::{
    loc::{Loc, PATH_MAPS},
    map::{Map, MapSpec, Terrain, TileType},
    side::{Side, SideArray},
    spells::Spell,
    tech::TechState,
    units::{Unit, Attack},
};


impl Board {
    /// Create a new empty board
    pub fn new(map: Map) -> Self {
        Self {
            map,
            pieces: HashMap::new(),
            reinforcements: SideArray::new(HashBag::new(), HashBag::new()),
            spells: SideArray::new(HashBag::new(), HashBag::new()),
            winner: None,
            state: BoardState::default(),
            bitboards: Bitboards::new(map.spec()),
        }
    }

    pub fn get_piece(&self, loc: &Loc) -> Option<&Piece> {
        self.pieces.get(loc)
    }

    pub fn get_piece_mut(&mut self, loc: Loc) -> Result<&mut Piece> {
        self.pieces.get_mut(&loc).context("No piece at loc")
    }

    /// Add a piece to the board
    pub fn add_piece(&mut self, piece: Piece) {
        debug_assert!(!self.pieces.contains_key(&piece.loc));
        self.bitboards.add_piece(&piece);
        self.pieces.insert(piece.loc, piece);
    }

    /// Remove a piece from the board
    pub fn remove_piece(&mut self, loc: &Loc) -> Option<Piece> {
        if let Some(piece) = self.pieces.remove(loc) {
            self.bitboards.remove_piece(loc, piece.side);
            Some(piece)
        } else {
            None
        }
    }

    pub fn bounce_piece(&mut self, piece: Piece) {
        self.remove_piece(&piece.loc);
        self.add_reinforcement(piece.unit, piece.side);
    }

    pub fn add_reinforcement(&mut self, unit: Unit, side: Side) {
        self.reinforcements[side].insert(unit);
    }

    pub fn can_move(&self, from_loc: &Loc, to_loc: &Loc, side_to_move: Side) -> Result<()> {
        let piece = self
            .get_piece(from_loc)
            .ok_or_else(|| anyhow!("No piece to move from {}", from_loc))?;

        ensure!(piece.side == side_to_move, "Cannot move opponent's piece");
        ensure!(!piece.state.borrow().moved, "Cannot move piece twice");

        let valid_moves = self.get_valid_move_hexes(*from_loc);
        ensure!(
            valid_moves.contains(to_loc),
            "No valid path from {} to {}",
            from_loc,
            to_loc
        );

        Ok(())
    }

    fn move_piece(&mut self, mut piece: Piece, to_loc: &Loc) {
        self.remove_piece(&piece.loc);
        piece.loc = *to_loc;
        piece.state.borrow_mut().moved = true;
        self.add_piece(piece);
    }

    fn try_attack(&self, attacker_loc: Loc, target_loc: Loc, side_to_move: Side) -> Result<(bool, bool)> {
        let attacker = self
            .get_piece(&attacker_loc)
            .ok_or_else(|| anyhow!("No attacker at {attacker_loc}"))?;

        ensure!(attacker.side == side_to_move, "Cannot attack with opponent's piece");

        let attacker_stats = attacker.unit.stats();
        let mut attacker_state = attacker.state.borrow_mut();

        ensure!(
            !attacker_state.moved || !attacker_stats.lumbering,
            "Lumbering piece cannot move and attack on the same turn"
        );

        ensure!(
            attacker_loc.dist(&target_loc) <= attacker_stats.range,
            "Attack range not large enough"
        );

        ensure!(
            attacker_state.attacks_used < attacker_stats.num_attacks,
            "Piece has already used all attacks"
        );

        let target = self
            .get_piece(&target_loc)
            .ok_or_else(|| anyhow!("No target at {target_loc}"))?;

        let target_stats = target.unit.stats();

        ensure!(target.side != side_to_move, "Cannot attack own piece");

        let attack = attacker_stats.attack;
        let mut damage_effect: i32 = 0;
        let mut bounce = false;

        match attack {
            Attack::Damage(damage) => {
                damage_effect = damage;
            }
            Attack::Deathtouch => {
                ensure!(!target_stats.necromancer, "Deathtouch cannot be used on necromancer");
                damage_effect = target_stats.defense;
            }
            Attack::Unsummon => {
                if target_stats.persistent {
                    damage_effect = 1
                } else {
                    bounce = true;
                }
            }
        }

        attacker_state.moved = true;
        attacker_state.attacks_used += 1;

        let mut target_state = target.state.borrow_mut();

        target_state.damage_taken += damage_effect;

        let remove = target_state.damage_taken >= target_stats.defense || bounce;

        Ok((remove, bounce))
    }

    fn path(&self, from: Loc, to: Loc) -> Result<Vec<Loc>> {
        let delta = &to - &from;
        let dist = from.dist(&to);

        if dist == 0 {
            return Ok(vec![from]);
        }

        if dist as usize >= PATH_MAPS.len() {
            bail!("No path found for distance {}", dist);
        }

        if let Some(paths) = PATH_MAPS[dist as usize].get(&delta) {
            if let Some(path_deltas) = paths.get(0) {
                let mut result_path = vec![from];
                let mut current = from;
                for d in path_deltas {
                    current = &current + d;
                    result_path.push(current);
                }
                return Ok(result_path);
            }
        }

        bail!("No path found for delta {:?}", delta)
    }

    fn spawn_piece(&mut self, side: Side, loc: Loc, unit: Unit) -> Result<()> {
        ensure!(
            self.pieces.get(&loc).is_none(),
            "Cannot spawn on occupied location"
        );
        let mut piece = Piece::new(unit, side, loc);
        piece.state = RefCell::new(PieceState::spawned());
        self.add_piece(piece);
        Ok(())
    }

    fn check_valid_move_cyclic(&self, side: Side, locs: &[Loc]) -> Result<()> {
        // Ensure all pieces in the cycle can move.
        for i in 0..locs.len() {
            let from = locs[i];
            let to = locs[(i + 1) % locs.len()];

            let piece = self.get_piece(&from).context("No piece to move in cycle")?;
            ensure!(piece.side == side, "Cannot move opponent's piece in cycle");
            ensure!(
                piece.state.borrow().can_act(),
                "Piece in cycle has already moved or attacked"
            );

            let dist = self.path(from, to).context("No path for cycle move")?.len() as i32 - 1;
            ensure!(
                dist <= piece.unit.stats().speed,
                "Move distance exceeds piece speed in cycle"
            );
        }

        Ok(())
    }

    fn move_pieces_cyclic(&mut self, side: Side, locs: &[Loc]) -> Result<()> {
        // TODO: check for valid paths
        self.check_valid_move_cyclic(side, locs)?;

        let pieces = locs.iter().map(|loc| 
            self.remove_piece(loc).unwrap()
        ).collect::<Vec<_>>();

        let cycled_locs = locs.iter().cycle().skip(1);
        
        for (mut piece, loc) in pieces.into_iter().zip(cycled_locs) {
            piece.loc = *loc;
            piece.state.borrow_mut().moved = true;
            self.add_piece(piece);
        }

        Ok(())
    }

    fn attack_piece(&mut self, attacker_loc: Loc, target_loc: Loc) -> Result<()> {
        let (damage_amount, bounce) = {
            let attacker = self.get_piece(&attacker_loc).context("No attacker")?;
            let target = self.get_piece(&target_loc).context("No target")?;
            let attack = attacker.unit.stats().attack;

            let mut damage_effect: i32 = 0;
            let mut bounce = false;

            match attack {
                Attack::Damage(damage) => {
                    damage_effect = damage;
                }
                Attack::Deathtouch => {
                    ensure!(
                        !target.unit.stats().necromancer,
                        "Deathtouch cannot be used on necromancer"
                    );
                    damage_effect = target.unit.stats().defense;
                }
                Attack::Unsummon => {
                    if target.unit.stats().persistent {
                        damage_effect = 1
                    } else {
                        bounce = true;
                    }
                }
            }
            (damage_effect, bounce)
        };

        self.get_piece_mut(attacker_loc)?.state.borrow_mut().exhausted = true;

        let target_is_dead = {
            let target_piece = self.get_piece_mut(target_loc)?;
            let mut target_state = target_piece.state.borrow_mut();
            target_state.damage_taken += damage_amount;
            target_state.damage_taken >= target_piece.unit.stats().defense
        };

        if target_is_dead || bounce {
            let target = self.remove_piece(&target_loc).unwrap();
            if bounce {
                self.add_reinforcement(target.unit, target.side);
            } else {
                // Not a bounce, so it's a "death".
                if target.unit.stats().necromancer {
                    self.winner = Some(target.side.opponent());
                } else {
                    // TODO: Handle money rebate for kills
                    self.add_reinforcement(target.unit, target.side);
                }
            }
        }

        Ok(())
    }

    fn resign(&mut self, side: Side) {
        self.winner = Some(side.opponent());
    }

    pub fn end_turn(
        &mut self,
        side_to_move: Side,
    ) -> Result<(i32, Option<Side>)> {
        for piece in self.pieces.values() {
            piece.state.borrow_mut().reset();
        }

        // Income
        let units_on_graveyards = self.units_on_graveyards(side_to_move);
        let soul_necromancer_income = if let Some(necro) = self.find_necromancer(side_to_move) {
            let necromancer = self.get_piece(&necro).unwrap();
            if matches!(necromancer.unit, Unit::BasicNecromancer | Unit::ArcaneNecromancer) {
                1
            } else {
                0
            }
        } else {
            0
        };

        let income = units_on_graveyards + 2 + soul_necromancer_income;

        // Check for graveyard control loss
        if self.units_on_graveyards(!side_to_move) >= Board::GRAVEYARDS_TO_WIN {
            self.winner = Some(!side_to_move);
        }

        self.state = match self.state {
            BoardState::Normal | BoardState::FirstTurn | BoardState::Reset2 => BoardState::Normal,
            BoardState::Reset0 => BoardState::Reset1,
            BoardState::Reset1 => BoardState::Reset2,
        };

        if let Some(winning_side) = self.winner {
            self.reset();
            self.state = if winning_side == side_to_move {
                BoardState::Reset0
            } else {
                BoardState::Reset1
            };
            
            return Ok((income, Some(winning_side)));
        } 

        Ok((income, None))
    }

    pub fn assign_spell(&mut self, spell: Spell, side: Side) -> Result<()> {
        self.spells[side].insert(spell);
        Ok(())
    }

    pub fn reset(&mut self) {
        let to_bounce = 
            self.pieces.drain()
                .filter(|(_, piece)| !piece.unit.stats().necromancer)
                .collect::<Vec<_>>();

        *self = Board::from_fen(Self::START_FEN, self.map).unwrap();

        for (_, piece) in to_bounce {
            self.add_reinforcement(piece.unit, piece.side);
        }
    }

    fn find_necromancer(&self, side: Side) -> Option<Loc> {
        self.pieces.values()
            .find(|p| p.side == side && p.unit.stats().necromancer)
            .map(|p| p.loc)
    }
}

impl Default for Board {
    fn default() -> Self {
        let map = Map::default();
        Self::from_fen(Self::START_FEN, map).expect("Failed to create default board")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::loc::Loc;
    use crate::core::units::Unit;

    #[test]
    fn test_board_pieces() {
        let map = Map::default();
        let mut board = Board::new(map);
        let loc = Loc::new(0, 0);
        let piece = Piece::new(Unit::Zombie, Side::S0, loc);
        board.add_piece(piece);
        assert_eq!(board.get_piece(&loc).unwrap().unit, Unit::Zombie);
        let removed = board.remove_piece(&loc).unwrap();
        assert_eq!(removed.unit, Unit::Zombie);
        assert!(board.get_piece(&loc).is_none());
    }

    #[test]
    fn test_board_fen_conversion() {
        let map = Map::default();
        let mut board = Board::new(map);

        board.add_piece(Piece::new(Unit::Zombie, Side::S0, Loc::new(0, 0)));
        board.add_piece(Piece::new(Unit::Zombie, Side::S1, Loc::new(1, 1)));
        board.add_piece(Piece::new(
            Unit::BasicNecromancer,
            Side::S0,
            Loc::new(2, 2),
        ));
        board.add_piece(Piece::new(
            Unit::BasicNecromancer,
            Side::S1,
            Loc::new(3, 3),
        ));

        let fen = board.to_fen();
        let new_board = Board::from_fen(&fen, map).unwrap();

        assert_eq!(board.pieces.len(), new_board.pieces.len());
        for (loc, piece) in &board.pieces {
            let new_piece = new_board.get_piece(loc).unwrap();
            assert_eq!(piece.unit, new_piece.unit);
            assert_eq!(piece.side, new_piece.side);
        }
    }

    #[test]
    fn test_fen_empty_board() {
        let map = Map::default();
        let board = Board::new(map);
        let fen = board.to_fen();
        let new_board = Board::from_fen(&fen, map).unwrap();
        assert!(new_board.pieces.is_empty());
    }

    #[test]
    fn test_fen_default_board() {
        let map = Map::default();
        let board = Board::new(map);
        let fen = board.to_fen();
        assert_eq!(fen, "0/0/0/0/0/0/0/0/0/0");
    }

    #[test]
    fn test_invalid_fen() {
        let map = Map::default();
        assert!(Board::from_fen("11/10", map).is_err());
        assert!(Board::from_fen("10/11", map).is_err());
        assert!(Board::from_fen("10/10/10/10/10/10/10/10/10/10", map).is_err());
        assert!(Board::from_fen("X/10", map).is_err());
        assert!(Board::from_fen("10/X", map).is_err());
    }

    #[test]
    fn test_board_reset() {
        let mut board = Board::default();

        board.add_piece(Piece::new(Unit::Initiate, Side::S0, Loc::new(0, 0)));
        board.add_piece(Piece::new(Unit::Skeleton, Side::S1, Loc::new(5, 6)));

        board.reset();

        assert_eq!(board.pieces.len(), 14);

        assert_eq!(board.reinforcements[Side::S0].len(), 7);
        assert_eq!(board.reinforcements[Side::S0].contains(&Unit::Zombie), 6);
        assert_eq!(board.reinforcements[Side::S0].contains(&Unit::Initiate), 1);

        assert_eq!(board.reinforcements[Side::S1].len(), 7);
        assert_eq!(board.reinforcements[Side::S1].contains(&Unit::Zombie), 6);
        assert_eq!(board.reinforcements[Side::S1].contains(&Unit::Skeleton), 1);
    }
}
