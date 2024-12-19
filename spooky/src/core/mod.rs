//! Core game representations and rules

pub mod action;
pub mod board;
pub mod convert;
pub mod game;
pub mod map;
pub mod side;
pub mod tech;
pub mod units;

pub use game::Game;
pub use board::Board;
pub use action::{Move, BoardAction};
pub use tech::{Tech, Techline, TechStatus};
pub use units::{Unit, UnitLabel, Attack};
pub use map::{Map, MapLabel, TileType, Loc, HexArray};
