use crate::ai::general_node::{GeneralNodeState, GeneralPreTurn};
use crate::ai::mcts::ChildGen;
use crate::core::Side;
use crate::core::{tech::TechAssignment, tech::TechState, GameConfig, GameState, SideArray};
use crate::heuristics::naive::{CombinedEnc, NaiveHeuristic};
use crate::heuristics::{GeneralHeuristic, Heuristic};
use rand::prelude::*;
use std::collections::HashSet;

impl<'a> GeneralHeuristic<'a, CombinedEnc<'a>> for NaiveHeuristic<'a> {
    type GeneralEnc = ();

    fn compute_enc(&self, _side: Side, _state: &TechState) -> Self::GeneralEnc {
        
    }

    fn update_enc(&self, enc: &Self::GeneralEnc, turn: &TechAssignment) -> Self::GeneralEnc {
        
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
