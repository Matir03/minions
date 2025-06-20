//! Strategic and tactical logic for the full game
//! Includes search, tech assignment, attacking, spawning, and board-level evaluations

pub mod attack;
pub mod blotto;
pub mod board; // New
pub mod eval;
pub mod game;
pub mod general;
pub mod mcts;
pub mod rng;
pub mod search;
pub mod spawn; // Will be refactored/removed


pub use search::{SearchTree, SearchResult};
pub use eval::Eval;
