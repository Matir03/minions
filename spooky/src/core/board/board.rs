use anyhow::{anyhow, bail, ensure, Context, Result};
use hashbag::HashBag;
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::{HashMap, HashSet},
};

use super::{
    actions::{AttackAction, SetupAction, SpawnAction},
    bitboards::Bitboards,
    definitions::{Board, BoardState},
    piece::{Piece, PieceState},
};
use crate::core::{
    loc::{Loc, PATH_MAPS},
    map::{Map, MapSpec, Terrain, TileType},
    side::{Side, SideArray},
    spells::Spell,
    tech::TechState,
    units::{Attack, Unit},
};

impl<'a> Board<'a> {
    /// Create a new empty board
    pub fn new(map: &'a Map) -> Self {
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

    pub fn get_piece(&self, loc: &Loc) -> Result<&Piece> {
        self.pieces.get(loc).context(format!("No piece at {}", loc))
    }

    pub fn get_piece_mut(&mut self, loc: &Loc) -> Result<&mut Piece> {
        self.pieces
            .get_mut(loc)
            .context(format!("No piece at {}", loc))
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
        let piece = self.get_piece(from_loc)?;

        ensure!(piece.side == side_to_move, "Cannot move opponent's piece");
        ensure!(!piece.state.moved, "Cannot move piece twice");
        ensure!(
            piece.state.attacks_used == 0,
            "Cannot move piece after attacking"
        );

        let valid_moves = self.get_valid_move_hexes(*from_loc);
        ensure!(
            valid_moves.contains(to_loc),
            "No valid path from {} to {}",
            from_loc,
            to_loc
        );

        Ok(())
    }

    // returns (removed, bounce)
    fn try_attack(
        &mut self,
        attacker_loc: Loc,
        target_loc: Loc,
        side_to_move: Side,
    ) -> Result<(bool, bool)> {
        let attacker = self.get_piece(&attacker_loc)?;

        ensure!(
            attacker.side == side_to_move,
            "Cannot attack with opponent's piece"
        );

        ensure!(attacker.state.can_attack(), "Piece is exhausted");

        let attacker_stats = attacker.unit.stats();
        let mut attacker_state = attacker.state.clone();

        ensure!(
            !attacker_state.moved || !attacker_stats.lumbering,
            format!(
                "Lumbering piece cannot move and attack on the same turn: {:#?}",
                attacker,
            )
        );

        ensure!(
            attacker_loc.dist(&target_loc) <= attacker_stats.range,
            "Attack range not large enough"
        );

        ensure!(
            attacker_state.attacks_used < attacker_stats.num_attacks,
            "Piece has already used all attacks"
        );

        let attack = attacker_stats.attack;

        let target = self.get_piece(&target_loc)?;

        let target_stats = target.unit.stats();

        ensure!(target.side != side_to_move, "Cannot attack own piece");

        let mut damage_effect: i32 = 0;
        let mut bounce = false;

        match attack {
            Attack::Damage(damage) => {
                damage_effect = damage;
            }
            Attack::Deathtouch => {
                ensure!(
                    !target_stats.necromancer,
                    "Deathtouch cannot be used on necromancer"
                );
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

        attacker_state.attacks_used += 1;

        let mut target_state = target.state.clone();
        target_state.damage_taken += damage_effect;

        let remove = target_state.damage_taken >= target_stats.defense || bounce;

        self.get_piece_mut(&attacker_loc)?.state = attacker_state;
        self.get_piece_mut(&target_loc)?.state = target_state;

        Ok((remove, bounce))
    }

    pub fn verify_path(&self, piece: &Piece, to: &Loc) -> Result<()> {
        let speed = piece.unit.stats().speed;
        let from = &piece.loc;
        let delta = to - from;

        let paths = PATH_MAPS[speed as usize].get(&delta).context(format!(
            "location {} too far for piece {} at {}",
            to, piece, from
        ))?;

        for path in paths {
            let path_valid = path.iter().all(|delta| {
                let loc = from + delta;
                self.can_pass_through(piece, loc)
            });

            if path_valid {
                return Ok(());
            }
        }

        Err(anyhow!(
            "No valid path found for piece {} from {} to {}",
            piece,
            from,
            to
        ))
    }

    pub fn can_pass_through(&self, piece: &Piece, loc: Loc) -> bool {
        let terrain_ok = match self.map.get_terrain(&loc) {
            Some(t) => t.allows(&piece.unit),
            None => true,
        };

        if !terrain_ok {
            return false;
        }

        let other = match self.get_piece(&loc) {
            Ok(p) => p,
            Err(_) => return true,
        };

        other.side == piece.side || piece.unit.stats().flying
    }

    pub fn spawn_piece(&mut self, side: Side, loc: Loc, unit: Unit) -> Result<()> {
        ensure!(
            self.pieces.get(&loc).is_none(),
            "spawn_piece called on occupied location"
        );
        let mut piece = Piece::new(unit, side, loc);
        piece.state = PieceState::spawned();
        self.add_piece(piece);
        Ok(())
    }

    pub fn check_valid_move(&self, side: Side, from: &Loc, to: &Loc) -> Result<()> {
        let piece = self.get_piece(from)?;
        ensure!(piece.side == side, "Cannot move opponent's piece");
        ensure!(
            piece.state.can_move(),
            "Piece has already moved or is exhausted"
        );
        ensure!(
            self.get_piece(to).is_err(),
            "Destination square is occupied"
        );

        self.verify_path(piece, to)
    }

    pub fn move_piece(&mut self, side: Side, from: &Loc, to: &Loc) -> Result<()> {
        self.check_valid_move(side, from, to)?;

        let mut piece = self.remove_piece(from).unwrap();

        piece.loc = *to;
        piece.state.moved = true;
        self.add_piece(piece);

        Ok(())
    }

    pub fn check_valid_move_cyclic(&self, side: Side, locs: &[Loc]) -> Result<()> {
        // Ensure all pieces in the cycle can move.
        for i in 0..locs.len() {
            let from = locs[i];
            let to = locs[(i + 1) % locs.len()];

            let piece = self.get_piece(&from)?;
            ensure!(piece.side == side, "Cannot move opponent's piece in cycle");
            ensure!(
                piece.state.can_move(),
                "Piece in cycle has already moved or is exhausted"
            );

            self.verify_path(piece, &to)?;
        }

        Ok(())
    }

    pub fn move_pieces_cyclic(&mut self, side: Side, locs: &[Loc]) -> Result<()> {
        self.check_valid_move_cyclic(side, locs)?;

        let pieces = locs
            .iter()
            .map(|loc| self.remove_piece(loc).unwrap())
            .collect::<Vec<_>>();

        let cycled_locs = locs.iter().cycle().skip(1);

        for (mut piece, loc) in pieces.into_iter().zip(cycled_locs) {
            piece.loc = *loc;
            piece.state.moved = true;
            self.add_piece(piece);
        }

        Ok(())
    }

    // returns rebate
    pub fn attack_piece(
        &mut self,
        attacker_loc: Loc,
        target_loc: Loc,
        side_to_move: Side,
    ) -> Result<i32> {
        let (removed, bounce) = self.try_attack(attacker_loc, target_loc, side_to_move)?;

        // self.get_piece_mut(&attacker_loc)
        //     .context("No attacker")?
        //     .state
        //     .exhausted = true;

        if removed {
            let target = self.remove_piece(&target_loc).unwrap();
            if bounce {
                self.add_reinforcement(target.unit, target.side);
            } else {
                // death
                if target.unit.stats().necromancer {
                    self.winner = Some(target.side.opponent());
                }

                return Ok(target.unit.stats().rebate);
            }
        }

        Ok(0)
    }

    pub fn resign(&mut self, side: Side) {
        self.winner = Some(side.opponent());
    }

    pub fn assign_spell(&mut self, spell: Spell, side: Side) -> Result<()> {
        self.spells[side].insert(spell);
        Ok(())
    }

    pub fn reset(&mut self) {
        let to_bounce = self
            .pieces
            .drain()
            .map(|(_, piece)| (piece.unit, piece.side))
            .filter(|(unit, _)| !unit.stats().necromancer)
            .collect::<HashSet<_>>();

        let new_board = Board::from_fen(Self::START_FEN, self.map).unwrap();
        self.pieces = new_board.pieces;
        self.winner = new_board.winner;
        self.state = new_board.state;
        self.bitboards = new_board.bitboards;

        for (unit, side) in to_bounce {
            self.add_reinforcement(unit, side);
        }

        for side in Side::all() {
            self.reinforcements[side].retain(|_, _| 1);
        }
    }

    pub fn find_necromancer(&self, side: Side) -> Option<Loc> {
        self.pieces
            .values()
            .find(|p| p.side == side && p.unit.stats().necromancer)
            .map(|p| p.loc)
    }
}

#[cfg(test)]
mod tests {
    use super::super::definitions::{Board, BoardState};
    use crate::core::board::Piece;
    use crate::core::loc::Loc;
    use crate::core::map::Map;
    use crate::core::side::Side;
    use crate::core::units::Unit;

    #[test]
    fn test_board_pieces() {
        let map = Map::default();
        let mut board = Board::new(&map);
        let loc = Loc::new(0, 0);
        let piece = Piece::new(Unit::Zombie, Side::Yellow, loc);
        board.add_piece(piece);
        assert!(board.get_piece(&loc).is_ok());
        let removed = board.remove_piece(&loc);
        assert!(removed.is_some());
        assert!(board.get_piece(&loc).is_err());
    }

    #[test]
    fn test_board_fen_conversion() {
        let map = Map::default();
        let board = Board::from_fen(Board::START_FEN, &map).unwrap();
        let fen = board.to_fen();
        assert_eq!(fen, Board::START_FEN);

        let board2 = Board::from_fen(&fen, &map).unwrap();
        assert_eq!(board.pieces.len(), board2.pieces.len());
        for (loc, piece) in &board.pieces {
            let piece2 = board2.get_piece(loc).unwrap();
            assert_eq!(piece.unit, piece2.unit);
            assert_eq!(piece.side, piece2.side);
        }
    }

    #[test]
    fn test_fen_empty_board() {
        let map = Map::default();
        let board = Board::new(&map);
        let fen = board.to_fen();
        let board2 = Board::from_fen(&fen, &map).unwrap();
        assert!(board2.pieces.is_empty());
    }

    #[test]
    fn test_fen_default_board() {
        let map = Map::default();
        let board = Board::from_fen(Board::START_FEN, &map).unwrap();
        let fen = board.to_fen();
        let board2 = Board::from_fen(&fen, &map).unwrap();
        assert_eq!(board.pieces.len(), board2.pieces.len());
    }

    #[test]
    fn test_invalid_fen() {
        let map = Map::default();
        let fen = "invalid fen";
        let result = Board::from_fen(fen, &map);
        assert!(result.is_err());
    }

    #[test]
    fn test_board_reset() {
        let map = Map::default();
        let mut board = Board::from_fen(Board::START_FEN, &map).unwrap();
        let num_pieces = board.pieces.len();
        board.winner = Some(Side::Yellow);
        board.end_turn(Side::Yellow).unwrap();
        assert_eq!(board.pieces.len(), num_pieces);
        assert_eq!(board.state, BoardState::Reset1);
    }
}
