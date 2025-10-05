use super::NaiveHeuristic;
use crate::core::{tech::TechAssignment, tech::TechState, GameConfig};
use crate::heuristics::GeneralHeuristic;

impl<'a> GeneralHeuristic<'a> for NaiveHeuristic {
    type GeneralEnc = ();

    fn compute_enc(&self, state: &TechState) -> Self::GeneralEnc {
        ()
    }

    fn update_enc(&self, enc: &Self::GeneralEnc, turn: &TechAssignment) -> Self::GeneralEnc {
        ()
    }
}
