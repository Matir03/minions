//! Strategic and tactical logic for the full game
//! Includes search, tech assignment, attacking, spawning, and board-level evaluations

pub mod combat;
pub mod blotto;
pub mod board; // New
pub mod eval;
pub mod game;
pub mod general;
pub mod mcts;
pub mod reposition;
pub mod rng;
pub mod spawn;
pub mod search;


pub use search::{SearchTree, SearchResult};
pub use eval::Eval;

#[cfg(test)]
mod tests;

