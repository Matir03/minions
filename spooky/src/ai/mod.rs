//! Strategic and tactical logic for the full game
//! Includes search, tech assignment, attacking, spawning, and board-level evaluations

pub mod mcts;
pub mod eval;
pub mod search;

pub mod game;
pub mod general;
pub mod attack;
pub mod blotto;
pub mod spawn;

pub mod rng;

pub use search::{Search, SearchResult};
pub use eval::Eval;
