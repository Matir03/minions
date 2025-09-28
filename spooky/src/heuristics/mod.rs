pub mod naive;
mod nn;

pub mod traits;

// Re-export key types
pub use traits::{Heuristic, LocalHeuristic, TechlineHeuristic, BoardHeuristic};
pub use naive::NaiveHeuristic;
