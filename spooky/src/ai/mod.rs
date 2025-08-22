//! Strategic and tactical logic for the full game
pub mod blotto;
pub mod captain;
pub mod eval;
pub mod game;
pub mod general;
pub mod mcts;
pub mod rng;
pub mod search;
pub mod graphviz;

// Re-export key types
pub use eval::Eval;
pub use search::{SearchResult, SearchTree};

#[cfg(test)]
pub mod tests;
