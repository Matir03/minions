//! MCTS Node representing the state of a single board and its turn processing (attack + spawn).

use std::cell::RefCell;
use std::vec::Vec as StdVec;

use bumpalo::{Bump, collections::Vec as BumpVec};
use rand::prelude::*;

use crate::core::{Board, Side, GameConfig, action::BoardAction, map::Map, tech::TechState, SideArray};
use crate::ai::mcts::{MCTSNode, NodeStats};
use crate::ai::search::SearchArgs;
use crate::ai::eval::Eval;

// It's expected that AttackStage and SpawnStage logic will be accessible.
// For now, let's assume helper functions or direct reimplementation/adaptation.
// Placeholder for AttackStage logic (will need to use actual implementation from ai/attack/mod.rs or similar)
use crate::ai::attack::AttackStage;
// Placeholder for SpawnStage logic (will need to use actual implementation from ai/spawn.rs or similar)
use crate::ai::spawn::SpawnStage;

pub struct BoardNode<'a> {
    pub stats: NodeStats,
    pub board: Board, // Current state of this specific board
    pub side_to_move: Side,
    pub delta_money: SideArray<i32>, // Money changes resulting from actions on this board in one step
    pub delta_points: SideArray<i32>,// Point changes resulting from actions on this board in one step
    pub children: StdVec<&'a RefCell<BoardNode<'a>>>,
    // Arena for allocating children, if BoardNode manages its own children directly
    // However, typically the main MCTS search/GameNode's arena would be used.
    // For now, assuming children are allocated by the caller of make_child (e.g. GameNode's MCTS loop)
    // and SearchArgs.arena is used.
}

pub type BoardNodeRef<'a> = &'a RefCell<BoardNode<'a>>;

impl<'a> BoardNode<'a> {
    pub fn new(board: Board, side_to_move: Side, _arena: &'a Bump) -> Self {
        Self {
            stats: NodeStats::new(),
            board,
            side_to_move,
            delta_money: SideArray::new(0, 0),
            delta_points: SideArray::new(0, 0),
            children: StdVec::new(),
        }
    }

    // Helper to apply a sequence of actions to a board
    // This is a simplified version; actual game logic might be more complex
    // and involve GameState/GameConfig for validation and effects.
    fn apply_actions_to_board(
        original_board: &Board,
        actions: &[BoardAction],
        side_to_move: Side,
        config: &GameConfig,
        tech_state: &TechState,
        current_delta_money: &mut SideArray<i32>,
        current_delta_points: &mut SideArray<i32>,
    ) -> Board {
        let mut new_board = original_board.clone();
        let map: &Map = &config.maps[0]; // Assuming Map is part of GameConfig. TODO: Revisit if multiple maps are used.

        // current_delta_money and current_delta_points are passed in and will be updated by Board::do_action

        for action in actions {
            // Board::do_action might need more context from GameState (e.g. tech_state)
            // For now, assuming it can work with the provided board and action primarily.
            // The full GameState is not available directly to BoardNode in this design,
            // only its own board. This might require Board::do_action to be more self-contained
            // or for BoardNode to receive more context.
            if let Err(e) = new_board.do_action(
                action.clone(),
                current_delta_money,  // Pass the mutable refs to be updated
                current_delta_points, // Pass the mutable refs to be updated
                tech_state,
                map,
                side_to_move,
            ) {
                // In a real scenario, decide how to handle action errors.
                // For MCTS, an invalid action sequence might lead to a very bad eval or pruning.
                eprintln!("[BoardNode] Error applying action {:?}: {}", action, e);
            }
        }
        new_board
    }
}

impl<'a> MCTSNode<'a> for BoardNode<'a> {
    // Etc is (money_allocation, tech_state_for_actions)
    type Args = (i32, TechState); // Now owned TechState

    fn stats(&self) -> &NodeStats {
        &self.stats
    }

    fn children(&self) -> &StdVec<&'a RefCell<Self>> {
        &self.children
    }

    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, etc: Self::Args) -> (bool, usize) {
        let (money_allocation, tech_state_owned) = etc;
        let tech_state = &tech_state_owned; // Use a reference to the owned TechState internally

        let mut turn_delta_money = SideArray::new(0, 0);
        let mut turn_delta_points = SideArray::new(0, 0);

        // 1. Attack Phase
        // Instantiate AttackStage or use its static methods
        let mut attack_stage = AttackStage::new(); // Assuming AttackStage can be default-constructed
        
        // generate_candidates might need &GameConfig or &Map
        // The current AttackStage in ai/attack/mod.rs takes (board, side, num_candidates)
        let num_attack_candidates = 3; // Example value
        let attack_action_candidates = attack_stage.generate_candidates(&self.board, self.side_to_move, num_attack_candidates);

        let selected_attack_actions = if attack_action_candidates.is_empty() {
            // Handle case with no attack actions (e.g., pass turn or specific no-op action)
            vec![BoardAction::EndPhase] // Or an empty vec if that's how no-ops are handled
        } else {
            // Select one set of attack actions (e.g., randomly or based on a heuristic)
            attack_action_candidates[rng.gen_range(0..attack_action_candidates.len())].clone()
        };

        let board_after_attack = Self::apply_actions_to_board(
            &self.board, 
            &selected_attack_actions, 
            self.side_to_move, 
            args.config,
            tech_state,
            &mut turn_delta_money,
            &mut turn_delta_points
        );

        // 2. Spawn Phase
        // SpawnStage::generate_spawns takes (board, side, money, rng)
        // The money_allocation is for the entire BoardNode's turn.
        // Assume it's primarily for spawning, as per current game mechanics.
        let spawn_actions = SpawnStage::generate_spawns(&board_after_attack, self.side_to_move, money_allocation, rng);
        
        let final_board_state = Self::apply_actions_to_board(
            &board_after_attack, 
            &spawn_actions, 
            self.side_to_move, 
            args.config,
            tech_state,
            &mut turn_delta_money, // Continue accumulating deltas
            &mut turn_delta_points // Continue accumulating deltas
        );
        
        // Create the new child BoardNode
        let mut new_child_node_inner = BoardNode::new(
            final_board_state,
            self.side_to_move, 
            args.arena,
        );
        // Store the accumulated deltas in the child node
        new_child_node_inner.delta_money = turn_delta_money;
        new_child_node_inner.delta_points = turn_delta_points;

        let new_child_node_ref = args.arena.alloc(RefCell::new(new_child_node_inner));

        let child_index = self.children.len();
        self.children.push(new_child_node_ref);
        (true, child_index)
    }

    fn update(&mut self, eval: &Eval) {
        self.stats.update(eval);
        // If BoardNode had a parent pointer for its own MCTS tree, it would update it here.
        // However, eval updates are typically propagated up the main GameNode tree.
    }
}
