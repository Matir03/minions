//! Strategic and tactical logic for the full game
pub mod blotto;
pub mod captain;
pub mod eval;
pub mod explore;
pub mod game_node;
pub mod general_node;
pub mod graphviz;
pub mod mcts;

// Re-export key types
pub use eval::Eval;
pub use explore::*;

#[cfg(test)]
pub mod tests;
