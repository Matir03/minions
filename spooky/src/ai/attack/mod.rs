pub mod combat;
pub mod prophecy;
pub mod solver;
mod constraints;

use std::collections::HashMap;
use std::cell::RefCell;
use crate::core::{Board, Side, action::BoardAction, SideArray};
use super::{
    eval::Eval,
    mcts::{MCTSNode, NodeStats},
    search::SearchArgs,
};
use combat::CombatGraph;
use prophecy::Prophecy;
use solver::CombatSolver;
use rand::prelude::*;

/// The attack stage generates combat moves using constraint satisfaction
pub struct AttackStage {
    pub generated_moves: Vec<Vec<BoardAction>>,
    pub move_constraints: Vec<HashMap<String, bool>>, // Simplified constraint tracking
}

impl AttackStage {
    pub fn new() -> Self {
        Self {
            generated_moves: Vec::new(),
            move_constraints: Vec::new(),
        }
    }

    /// Generate candidate moves for the attack stage
    pub fn generate_candidates(&mut self, board: &Board, side: Side, num_candidates: usize) -> Vec<Vec<BoardAction>> {
        let mut candidates = Vec::new();

        // Build combat graph
        let graph = board.combat_graph();

        println!("[DEBUG] Board has {} pieces:", board.pieces.len());
        for (loc, piece) in &board.pieces {
            println!("[DEBUG] Piece at {:?}: {:?}", loc, piece.unit);
        }

        for i in 0..num_candidates {
            let mut prophecy = Prophecy::generate_basic(board, side);
            if i > 0 {
                prophecy = prophecy.add_noise(0.2);
            }

            // Use the constraint solver
            let mut solver = crate::ai::attack::constraints::ConstraintSolver::new(&graph);
            solver.add_all_constraints(&graph, board);
            solver.apply_prophecy(&prophecy);

            if solver.is_satisfiable(board) {
                let actions = solver.generate_move(board, side);
                if !actions.is_empty() {
                    candidates.push(actions);
                }
            }
        }

        if candidates.is_empty() {
            println!("[DEBUG] No candidate moves found, generating dummy null move");
            candidates.push(vec![BoardAction::EndPhase]);
        }

        println!("[DEBUG] AttackStage: generated {} candidate moves", candidates.len());
        self.generated_moves = candidates.clone();
        candidates
    }

    /// Evaluate a position after applying a move
    pub fn evaluate_position(&self, board: &Board, side: Side) -> f32 {
        let mut score = 0.0;

        // Count pieces by value
        for (_, piece) in &board.pieces {
            let piece_value = piece.unit.stats().cost as f32;
            let health_ratio = {
                let stats = piece.unit.stats();
                let damage = piece.state.borrow().damage_taken;
                (stats.defense - damage) as f32 / stats.defense as f32
            };

            let adjusted_value = piece_value * health_ratio;

            if piece.side == side {
                score += adjusted_value;
            } else {
                score -= adjusted_value;
            }
        }

        // Bonus for controlling graveyards (simplified)
        // TODO: Implement proper graveyard counting

        score
    }

    /// Refine a candidate move
    pub fn refine_move(&self, board: &Board, side: Side, base_move: &[BoardAction]) -> Option<Vec<BoardAction>> {
        // For now, just return the base move
        // In a full implementation, this would try to improve the move
        Some(base_move.to_vec())
    }

    /// Get the best move from generated candidates
    pub fn get_best_move(&self, board: &Board, side: Side) -> Option<Vec<BoardAction>> {
        if self.generated_moves.is_empty() {
            return None;
        }

        // Simple heuristic: pick the move that attacks the most valuable pieces
        let mut best_move = None;
        let mut best_score = f32::NEG_INFINITY;

        for move_actions in &self.generated_moves {
            // Simulate the move on a copy of the board
            let mut board_copy = board.clone();

            // Apply the move (simplified - just check if it's valid)
            let mut valid = true;
            for action in move_actions {
                match action {
                    BoardAction::Attack { attacker_loc, target_loc } => {
                        if let Some(attacker) = board_copy.get_piece(attacker_loc) {
                            if attacker.side != side {
                                valid = false;
                                break;
                            }
                        } else {
                            valid = false;
                            break;
                        }

                        if let Some(target) = board_copy.get_piece(target_loc) {
                            if target.side == side {
                                valid = false;
                                break;
                            }
                        } else {
                            valid = false;
                            break;
                        }
                    }
                    BoardAction::Move { from_loc, to_loc } => {
                        if let Some(piece) = board_copy.get_piece(from_loc) {
                            if piece.side != side {
                                valid = false;
                                break;
                            }
                        } else {
                            valid = false;
                            break;
                        }
                    }
                    _ => {
                        // Skip other action types for now
                    }
                }
            }

            if valid {
                let score = self.evaluate_position(&board_copy, side);
                if score > best_score {
                    best_score = score;
                    best_move = Some(move_actions.clone());
                }
            }
        }

        best_move
    }
}