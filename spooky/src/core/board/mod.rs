//! Board representation and rules

pub mod actions;
pub mod bitboards;
pub mod definitions;
pub mod fen;
pub use bitboards::{Bitboard, BitboardOps, Bitboards};

pub mod piece;

pub use definitions::{Board, BoardState};
pub use piece::{Modifiers, Piece, PieceState};

use self::actions::{AttackAction, SetupAction, SpawnAction};
use crate::core::{loc::GRID_LEN, side};
use anyhow::{anyhow, bail, ensure, Context, Result};
use hashbag::HashBag;
use std::{cell::RefCell, collections::HashMap};

use super::{
    loc::{Loc, PATH_MAPS},
    map::{Map, MapSpec, Terrain, TileType},
    side::{Side, SideArray},
    spells::Spell,
    tech::TechState,
    units::{Attack, Unit},
};

mod board;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::loc::Loc;
    use crate::core::units::Unit;

    #[test]
    fn test_board_pieces() {
        let map = Map::default();
        let mut board = Board::new(&map);
        let loc = Loc::new(0, 0);
        let piece = Piece::new(Unit::Zombie, Side::Yellow, loc);
        board.add_piece(piece);
        assert_eq!(board.get_piece(&loc).unwrap().unit, Unit::Zombie);
        let removed = board.remove_piece(&loc).unwrap();
        assert_eq!(removed.unit, Unit::Zombie);
        assert!(board.get_piece(&loc).is_err());
    }

    #[test]
    fn test_board_fen_conversion() {
        let map = Map::default();
        let mut board = Board::new(&map);

        board.add_piece(Piece::new(Unit::Zombie, Side::Yellow, Loc::new(0, 0)));
        board.add_piece(Piece::new(Unit::Zombie, Side::Blue, Loc::new(1, 1)));
        board.add_piece(Piece::new(
            Unit::BasicNecromancer,
            Side::Yellow,
            Loc::new(2, 2),
        ));
        board.add_piece(Piece::new(
            Unit::BasicNecromancer,
            Side::Blue,
            Loc::new(3, 3),
        ));

        let fen = board.to_fen();
        let new_board = Board::from_fen(&fen, &map).unwrap();

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
        let board = Board::new(&map);
        let fen = board.to_fen();
        let new_board = Board::from_fen(&fen, &map).unwrap();
        assert!(new_board.pieces.is_empty());
    }

    #[test]
    fn test_fen_default_board() {
        let map = Map::default();
        let board = Board::new(&map);
        let fen = board.to_fen();
        assert_eq!(fen, "f|||||0/0/0/0/0/0/0/0/0/0");
    }

    #[test]
    fn test_invalid_fen() {
        let map = Map::default();
        assert!(Board::from_fen("11/10", &map).is_err());
        assert!(Board::from_fen("10/11", &map).is_err());
        assert!(Board::from_fen("10/10/10/10/10/10/10/10/10/10", &map).is_err());
        assert!(Board::from_fen("X/10", &map).is_err());
        assert!(Board::from_fen("10/X", &map).is_err());
    }

    #[test]
    fn test_board_reset() {
        let map = Map::default();
        let mut board = Board::new(&map);

        board.add_piece(Piece::new(Unit::Initiate, Side::Yellow, Loc::new(0, 0)));
        board.add_piece(Piece::new(Unit::Skeleton, Side::Blue, Loc::new(5, 6)));

        board.reset();

        assert_eq!(board.pieces.len(), 14);

        assert_eq!(board.reinforcements[Side::Yellow].len(), 1);
        assert_eq!(
            board.reinforcements[Side::Yellow].contains(&Unit::Initiate),
            1
        );

        assert_eq!(board.reinforcements[Side::Blue].len(), 1);
        assert_eq!(
            board.reinforcements[Side::Blue].contains(&Unit::Skeleton),
            1
        );
    }
}
