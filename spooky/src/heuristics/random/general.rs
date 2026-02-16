use crate::ai::general_node::GeneralPreTurn;
use crate::core::{tech::TechAssignment, tech::TechState, Side};
use crate::heuristics::random::{CombinedEnc, RandomHeuristic};
use crate::heuristics::{GeneralHeuristic, Heuristic};
use rand::prelude::*;

impl<'a> GeneralHeuristic<'a, CombinedEnc<'a>> for RandomHeuristic<'a> {
    type GeneralEnc = ();

    fn compute_enc(&self, _side: Side, _state: &TechState) -> Self::GeneralEnc {
        
    }

    fn update_enc(&self, _enc: &Self::GeneralEnc, _turn: &TechAssignment) -> Self::GeneralEnc {
        
    }

    fn compute_general_pre_turn(
        &self,
        rng: &mut impl Rng,
        _shared: &CombinedEnc<'a>,
        _enc: &Self::GeneralEnc,
    ) -> GeneralPreTurn {
        let techline = &self.config.techline;
        let weights = techline.techs.iter().map(|_| rng.random()).collect();

        GeneralPreTurn { weights }
    }
}
