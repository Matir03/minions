//! Core game representations and rules

pub mod action;
pub mod blotto;
pub mod board;
pub mod convert;
pub mod display;
pub mod fen;
pub mod game;
pub mod loc;
pub mod map;
pub mod side;
pub mod spells;
pub mod tech;
pub mod units;
pub mod utils;

pub use action::{GameAction, GameTurn};
pub use board::Board;
pub use convert::{FromIndex, ToIndex};
pub use display::*;
pub use game::{GameConfig, GameState};
pub use loc::Loc;
pub use map::{HexGrid, Map, MapSpec, TileType};
pub use side::{Side, SideArray};
pub use spells::{Spell, SpellCast};
pub use tech::{Tech, TechStatus, Techline};
pub use units::{Attack, Unit, UnitStats};
pub use utils::Sigmoid;

pub use blotto::Blotto;
