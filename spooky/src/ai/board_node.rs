//! MCTS Node representing the state of a single board and its turn processing (attack + spawn).

use std::cell::RefCell;
use std::vec::Vec as StdVec;
use std::ops::AddAssign;

use bumpalo::{Bump, collections::Vec};
use rand::prelude::*;

use crate::core::{Board, Side, GameConfig, action::BoardAction, map::Map, tech::TechState, SideArray};
use crate::ai::mcts::{NodeState, NodeStats, MCTSNode};
use crate::ai::search::SearchArgs;
use crate::ai::eval::Eval;

// Placeholder for AttackStage logic
use crate::ai::attack::AttackStage;
// Placeholder for SpawnStage logic
use crate::ai::spawn::SpawnStage;

/// Represents the actions taken during a single board's turn phase.
#[derive(Clone, Debug)]
pub struct BoardTurn {
    pub attack_actions: StdVec<BoardAction>,
    pub spawn_actions: StdVec<BoardAction>,
}

#[derive(Clone)]
pub struct BoardNodeState {
    pub board: Board,
    pub side_to_move: Side,
    pub delta_money: SideArray<i32>,
    pub delta_points: SideArray<i32>,
}

impl BoardNodeState {
    pub fn new(board: Board, side_to_move: Side) -> Self {
        Self {
            board,
            side_to_move,
            delta_money: SideArray::new(0, 0),
            delta_points: SideArray::new(0, 0),
        }
    }

    fn apply_actions_to_board(
        original_board: &Board,
        actions: &[BoardAction],
        side_to_move: Side,
        config: &GameConfig,
        tech_state: &TechState,
        current_board_idx: usize,
    ) -> (Board, SideArray<i32>, SideArray<i32>) {
        let mut new_board = original_board.clone();
        let map: &Map = &config.maps.get(current_board_idx).unwrap_or_else(|| &config.maps[0]);
        let mut turn_delta_money = SideArray::new(0, 0);
        let mut turn_delta_points = SideArray::new(0, 0);

        for action in actions {
            if let Err(e) = new_board.do_action(
                action.clone(),
                &mut turn_delta_money,
                &mut turn_delta_points,
                tech_state,
                map,
                side_to_move,
            ) {
                eprintln!("[BoardNodeState] Error applying action {:?}: {}", action, e);
            }
        }
        (new_board, turn_delta_money, turn_delta_points)
    }
}

impl NodeState<BoardTurn> for BoardNodeState {
    type Args = (i32, TechState, GameConfig, usize); // Added usize for current_board_idx

    fn propose_move(&self, rng: &mut impl Rng, args: &Self::Args) -> (BoardTurn, Self) {
        let (money_allocation, tech_state, config, current_board_idx) = args; // Added current_board_idx

        let mut proposed_turn_delta_money = SideArray::new(0, 0);
        let mut proposed_turn_delta_points = SideArray::new(0, 0);

        let mut attack_stage = AttackStage::new();
        let num_attack_candidates = 3;
        let attack_action_candidates = attack_stage.generate_candidates(&self.board, self.side_to_move, num_attack_candidates);

        let selected_attack_actions = if attack_action_candidates.is_empty() {
            StdVec::new()
        } else {
            attack_action_candidates[rng.gen_range(0..attack_action_candidates.len())].clone()
        };

        let (board_after_attack, attack_delta_money, attack_delta_points) = Self::apply_actions_to_board(
            &self.board,
            &selected_attack_actions,
            self.side_to_move,
            config,
            &tech_state,
            *current_board_idx // Added current_board_idx
        );
        proposed_turn_delta_money.add_assign(&attack_delta_money);
        proposed_turn_delta_points.add_assign(&attack_delta_points);

        let spawn_actions = SpawnStage::generate_spawns(&board_after_attack, self.side_to_move, *money_allocation, rng);

        let (final_board_state, spawn_delta_money, spawn_delta_points) = Self::apply_actions_to_board(
            &board_after_attack,
            &spawn_actions,
            self.side_to_move,
            config,
            &tech_state,
            *current_board_idx // Added current_board_idx
        );
        proposed_turn_delta_money.add_assign(&spawn_delta_money);
        proposed_turn_delta_points.add_assign(&spawn_delta_points);

        let turn_taken = BoardTurn {
            attack_actions: selected_attack_actions,
            spawn_actions,
        };

        let next_board_node_state = BoardNodeState {
            board: final_board_state,
            side_to_move: self.side_to_move,
            delta_money: proposed_turn_delta_money,
            delta_points: proposed_turn_delta_points,
        };

        (turn_taken, next_board_node_state)
    }
}

pub type BoardNode<'a> = MCTSNode<'a, BoardNodeState, BoardTurn>;
pub type BoardNodeRef<'a> = &'a RefCell<BoardNode<'a>>;