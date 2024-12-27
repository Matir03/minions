//! Core game representations and rules

pub mod action;
pub mod bitboards;
pub mod board;
pub mod convert;
pub mod display;
pub mod game;
pub mod loc;
pub mod map;
pub mod side;
pub mod spells;
pub mod tech;
pub mod units;

pub use game::{GameConfig, GameState};
pub use board::Board;
pub use action::{Turn, BoardAction, GameAction};
pub use tech::{Tech, Techline, TechStatus};
pub use units::{UnitStats, Unit, Attack};
pub use map::{MapSpec, Map, TileType, HexGrid};
pub use spells::{Spell, SpellCast};
pub use loc::Loc;
pub use side::Side;
pub use display::*;