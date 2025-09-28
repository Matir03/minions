//! Strategic and tactical logic for the full game
pub mod blotto;
pub mod captain;
pub mod eval;
pub mod game;
pub mod general;
pub mod graphviz;
pub mod mcts;
pub mod search;

// Re-export key types
pub use eval::Eval;
pub use search::{NaiveSearchTree, SearchResult, SearchTree};

#[cfg(test)]
pub mod tests;
