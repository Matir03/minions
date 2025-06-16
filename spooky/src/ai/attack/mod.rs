pub mod combat;
pub mod prophecy;
pub mod solver;

use std::collections::HashMap;
use std::cell::RefCell;
use crate::core::{Board, Side, action::BoardAction, SideArray};
use super::{eval::Eval, mcts::{MCTSNode, NodeStats, Stage}, search::SearchArgs, spawn::{SpawnNode, SpawnNodeRef}};
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

        // Generate multiple prophecies
        for i in 0..num_candidates {
            let mut prophecy = Prophecy::generate_basic(board, side);

            // Add noise for variation
            if i > 0 {
                prophecy = prophecy.add_noise(0.2);
            }

            // Solve constraints
            let mut solver = CombatSolver::new(graph.clone(), prophecy);

            if solver.solve() {
                let actions = solver.generate_move(board, side);

                // Basic validation
                if !actions.is_empty() && solver.is_valid_move(board, side) {
                    candidates.push(actions);
                }
            }
        }

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

/// MCTS node for the attack stage
pub struct AttackNode<'a> {
    pub stats: NodeStats,
    pub board: Board,
    pub delta_points: SideArray<i32>,
    pub delta_money: SideArray<i32>,
    pub combat_graph: CombatGraph,
    pub attack_stage: AttackStage,
    pub parent: Option<SpawnNodeRef<'a>>,
    pub children: bumpalo::collections::Vec<'a, SpawnNodeRef<'a>>,
}

pub type AttackNodeRef<'a> = &'a RefCell<AttackNode<'a>>;

impl<'a> AttackNode<'a> {
    pub fn new(board: Board, arena: &'a bumpalo::Bump) -> Self {
        let combat_graph = board.combat_graph();
        let attack_stage = AttackStage::new();

        Self {
            stats: NodeStats::new(),
            board,
            delta_points: SideArray::new(0, 0),
            delta_money: SideArray::new(0, 0),
            combat_graph,
            attack_stage,
            children: bumpalo::collections::Vec::new_in(arena),
            parent: None,
        }
    }
}

impl<'a> MCTSNode<'a> for AttackNode<'a> {
    type Child = SpawnNode<'a>;
    type Etc = ();

    fn stats(&self) -> &NodeStats {
        &self.stats
    }

    fn children(&self) -> &bumpalo::collections::Vec<'a, SpawnNodeRef<'a>> {
        &self.children
    }

    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, _etc: ()) -> (bool, usize) {
        // Generate candidate moves
        let candidates = self.attack_stage.generate_candidates(&self.board, Side::S0, 3);

        if candidates.is_empty() {
            return (false, 0);
        }

        // For now, just pick a random candidate
        let selected_move = rng.gen_range(0..candidates.len());

        // TODO: Apply the move to create a new board state
        // For now, just return the index
        (true, selected_move)
    }

    fn update(&mut self, eval: &Eval) {
        self.stats.update(eval);

        if let Some(parent) = &self.parent {
            parent.borrow_mut().update(eval);
        }
    }
}