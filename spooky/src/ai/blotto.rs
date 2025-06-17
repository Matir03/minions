//! Resource allocation across boards

use rand::prelude::*;
use crate::core::{GameState, Side, Board, action::BoardAction};

use super::{general::GeneralNode, game::GameNode, spawn::{SpawnNode, SpawnNodeRef}};
use super::{eval::Eval, mcts::{MCTSNode, NodeStats}, search::SearchArgs};
use std::cell::RefCell;
use std::vec::Vec;
use bumpalo::Bump;

type Blotto = Vec<i32>;

pub fn blotto(node: &GameNode, general_next: &GeneralNode, boards_next: &Vec<SpawnNodeRef>, rng: &mut impl Rng) -> Blotto {
    let side = node.state.side_to_move;
    let money = node.state.money[side] + general_next.delta_money[side];
    let n = node.state.boards.len();

    let mut blotto = vec![0; n];

    for dollar in 0..money {
        let board = rng.gen_range(0..=n);
        if board < n {
            blotto[board] += 1;
        }
    }

    blotto
}

// TODO: conv blotto

pub struct BlottoStage;

impl BlottoStage {
    /// Allocate money to each board for the given side. Returns a Vec of allocations (one per board).
    pub fn allocate(game_state: &GameState, side: Side) -> Vec<i32> {
        let num_boards = game_state.boards.len();
        let total_money = game_state.money[side];
        if num_boards == 0 {
            return vec![];
        }
        let base = total_money / num_boards as i32;
        let mut allocations = vec![base; num_boards];
        let mut remainder = total_money - base * num_boards as i32;
        // Distribute any remainder
        for i in 0..num_boards {
            if remainder > 0 {
                allocations[i] += 1;
                remainder -= 1;
            }
        }
        allocations
    }
}

/// MCTS node for the blotto stage
pub struct BlottoNode<'a> {
    pub stats: NodeStats,
    pub boards: Vec<Board>,
    pub allocations: Vec<i32>,
    pub side: Side,
    pub parent: Option<super::attack::AttackNodeRef<'a>>,
    pub children: Vec<SpawnNodeRef<'a>>,
}

pub type BlottoNodeRef<'a> = &'a RefCell<BlottoNode<'a>>;

impl<'a> BlottoNode<'a> {
    pub fn new(boards: Vec<Board>, side: Side, arena: &'a Bump) -> Self {
        let allocations = BlottoStage::allocate(&GameState {
            boards: boards.clone(),
            side_to_move: side,
            board_points: crate::core::SideArray::new(0, 0),
            tech_state: crate::core::tech::TechState::new(),
            money: crate::core::SideArray::new(12, 12), // Default money
        }, side);

        Self {
            stats: NodeStats::new(),
            boards,
            allocations,
            side,
            parent: None,
            children: Vec::new(),
        }
    }
}

impl<'a> MCTSNode<'a> for BlottoNode<'a> {
    type Child = SpawnNode<'a>;
    type Etc = i32; // blotto allocation

    fn stats(&self) -> &NodeStats {
        &self.stats
    }

    fn children(&self) -> &Vec<SpawnNodeRef<'a>> {
        &self.children
    }

    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, blotto: i32) -> (bool, usize) {
        // Create spawn nodes for each board with the blotto allocation
        let mut new_children = Vec::new();

        for (board_idx, board) in self.boards.iter().enumerate() {
            let spawn_actions = super::spawn::SpawnStage::generate_spawns(board, self.side, blotto, rng);

            // Apply spawn actions to create new board state
            let mut new_board = board.clone();
            for action in spawn_actions {
                // Simplified: just apply the spawn action
                if let crate::core::action::BoardAction::Spawn { spawn_loc, unit } = action {
                    let piece = crate::core::board::Piece {
                        loc: spawn_loc,
                        side: self.side,
                        unit,
                        modifiers: crate::core::board::Modifiers::default(),
                        state: crate::core::board::PieceState::default().into(),
                    };
                    new_board.add_piece(piece);
                }
            }

            let spawn_node = args.arena.alloc(RefCell::new(
                SpawnNode::new(new_board, self.side, args.arena)
            ));
            let spawn_node_ref: &'a RefCell<SpawnNode<'a>> = spawn_node;
            new_children.push(spawn_node_ref);
        }

        self.children = new_children;

        // Ensure at least one child exists
        if self.children.is_empty() {
            println!("[DEBUG] make_child: creating dummy node for BlottoNode");
            let dummy_node = args.arena.alloc(RefCell::new(SpawnNode::new(Board::default(), self.side, args.arena)));
            self.children.push(dummy_node);
            return (true, 0);
        }

        (true, 0) // Return first child for now
    }

    fn update(&mut self, eval: &Eval) {
        self.stats.update(eval);

        if let Some(parent) = &self.parent {
            parent.borrow_mut().update(eval);
        }
    }
}