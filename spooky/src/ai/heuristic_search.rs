//! Generic search implementation that uses the Heuristic trait
//!
//! This demonstrates how the AI orchestration can now work with any heuristic
//! that implements the Heuristic trait, allowing for easy swapping between
//! different heuristic implementations (naive, neural networks, etc.)

use crate::{
    ai::{eval::Eval, search::SearchArgs},
    core::{GameConfig, GameState, GameTurn},
    heuristics::{traits::Heuristic, NaiveHeuristic},
};

use bumpalo::Bump;
use rand::prelude::*;
use std::cell::RefCell;

pub struct GenericSearchResult {
    pub eval: Eval,
    pub best_turn: GameTurn,
    pub nodes_explored: u32,
}

pub struct GenericSearchTree<'a, H: Heuristic<'a>> {
    pub args: SearchArgs<'a>,
    pub rng: StdRng,
    pub heuristic: H,
    pub _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, H: Heuristic<'a>> GenericSearchTree<'a, H> {
    pub fn new(config: &'a GameConfig, heuristic: H, arena: &'a Bump) -> Self {
        use crate::utils::make_rng;
        
        Self {
            args: SearchArgs { config, arena },
            rng: make_rng(),
            heuristic,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn evaluate_position(&self, state: &GameState<'a>) -> Eval {
        // Use the generic heuristic to evaluate the position
        
        // Compute encodings for techline and boards
        let tech_acc = self.heuristic.compute_acc(&state.tech_state);
        let tech_enc = self.heuristic.compute_enc(&tech_acc);
        
        let mut board_encs = Vec::new();
        for board in &state.boards {
            let board_acc = self.heuristic.compute_acc(board);
            let board_enc = self.heuristic.compute_enc(&board_acc);
            board_encs.push(board_enc);
        }
        
        let board_enc_refs: Vec<_> = board_encs.iter().collect();
        
        // Compute shared data
        let shared = self.heuristic.compute_shared(state, &tech_enc, &board_enc_refs);
        
        // Use the heuristic to compute evaluation
        self.heuristic.compute_eval(&shared)
    }
    
    pub fn suggest_money_distribution(&self, state: &GameState<'a>) -> Vec<Vec<i32>> {
        // Use the generic heuristic to suggest money distributions
        
        // Compute encodings for techline and boards
        let tech_acc = self.heuristic.compute_acc(&state.tech_state);
        let tech_enc = self.heuristic.compute_enc(&tech_acc);
        
        let mut board_encs = Vec::new();
        for board in &state.boards {
            let board_acc = self.heuristic.compute_acc(board);
            let board_enc = self.heuristic.compute_enc(&board_acc);
            board_encs.push(board_enc);
        }
        
        let board_enc_refs: Vec<_> = board_encs.iter().collect();
        
        // Compute shared data
        let shared = self.heuristic.compute_shared(state, &tech_enc, &board_enc_refs);
        
        // Use the heuristic to compute blotto distributions
        self.heuristic.compute_blottos(&shared)
    }
}

// Example usage function showing how different heuristics can be used interchangeably
pub fn example_usage(config: &GameConfig, state: GameState, arena: &Bump) -> GenericSearchResult {
    // Create a naive heuristic
    let naive_heuristic = NaiveHeuristic::new(config);
    
    // Create the search tree with the naive heuristic using the new API
    let mut search_tree = super::search::SearchTree::new(config, state.clone(), arena, naive_heuristic);
    
    // Run a few search iterations
    for _ in 0..10 {
        search_tree.explore();
    }
    
    // Get the search result
    let result = search_tree.result();
    
    println!("Position evaluation: {:?}", result.eval);
    println!("Best turn: {:?}", result.best_turn);
    println!("Nodes explored: {}", result.nodes_explored);
    
    GenericSearchResult {
        eval: result.eval,
        best_turn: result.best_turn,
        nodes_explored: result.nodes_explored,
    }
}

// This demonstrates that in the future, we could easily swap in different heuristics:
// pub fn example_with_neural_network(config: &GameConfig, state: GameState, arena: &Bump) -> GenericSearchResult {
//     let neural_heuristic = NeuralNetworkHeuristic::new(config);
//     let search_tree = GenericSearchTree::new(config, neural_heuristic, arena);
//     // ... rest would be identical
// }
