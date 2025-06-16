use super::{
    attack::{AttackNode, AttackNodeRef}, eval::Eval, mcts::{MCTSNode, NodeStats, Stage}, search::SearchArgs
};
use crate::core::{Board, SideArray, Side, units::Unit, action::BoardAction};

use std::cell::RefCell;

use rand::prelude::*;

use std::vec::Vec;

/// MCTS node for the spawn stage
pub struct SpawnNode<'a> {
    pub stats: NodeStats,
    pub board: Board,
    pub side: Side,
    pub parent: Option<super::blotto::BlottoNodeRef<'a>>,
    pub children: Vec<super::general::GeneralNodeRef<'a>>,
}

pub type SpawnNodeRef<'a> = &'a RefCell<SpawnNode<'a>>;

impl<'a> SpawnNode<'a> {
    pub fn new(board: Board, side: Side, arena: &'a bumpalo::Bump) -> Self {
        Self {
            stats: NodeStats::new(),
            board,
            side,
            parent: None,
            children: Vec::new(),
        }
    }
// Helper to check if a node is a dummy node (for future reference)
impl<'a> SpawnNode<'a> {
    pub fn is_dummy(&self) -> bool {
        self.children.is_empty() && self.parent.is_none()
    }
}

impl<'a> MCTSNode<'a> for SpawnNode<'a> {
    type Child = super::general::GeneralNode<'a>;
    type Etc = i32; // blotto allocation

    fn stats(&self) -> &NodeStats {
        &self.stats
    }

    fn children(&self) -> &Vec<super::general::GeneralNodeRef<'a>> {
        &self.children
    }

    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, blotto: i32) -> (bool, usize) {
        // Generate spawn actions for this board
        let spawn_actions = SpawnStage::generate_spawns(&self.board, self.side, blotto, rng);

        // Apply spawn actions to create new board state
        let mut new_board = self.board.clone();
        for action in spawn_actions {
            if let BoardAction::Spawn { spawn_loc, unit } = action {
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

        // Create a general node for the next turn
        let next_side = !self.side;
        let general_node = args.arena.alloc(RefCell::new(
            super::general::GeneralNode::new(
                next_side,
                crate::core::tech::TechState::new(), // TODO: pass actual tech state
                crate::core::SideArray::new(0, 0), // TODO: pass actual money delta
                args.arena
            )
        ));

        let child_index = self.children.len();
        self.children.push(general_node);

        // Ensure at least one child exists
        if self.children.is_empty() {
            println!("[DEBUG] make_child: creating dummy node for SpawnNode");
            let dummy_node = args.arena.alloc(RefCell::new(super::general::GeneralNode::new(self.side, crate::core::tech::TechState::new(), crate::core::SideArray::new(0, 0), args.arena)));
            self.children.push(dummy_node);
            return (true, self.children.len() - 1);
        }

        (true, child_index)
    }

    fn update(&mut self, eval: &Eval) {
        self.stats.update(eval);

        if let Some(parent) = &self.parent {
            parent.borrow_mut().update(eval);
        }
    }
}

pub struct SpawnStage;

impl SpawnStage {
    /// Generate spawn actions for a board given the allocated money.
    pub fn generate_spawns(board: &Board, side: Side, money: i32, rng: &mut impl Rng) -> Vec<BoardAction> {
        let mut actions = Vec::new();
        if money <= 0 {
            return actions;
        }
        // Find the cheapest unit available
        let mut cheapest_unit: Option<Unit> = None;
        let mut cheapest_cost = i32::MAX;
        for unit in [Unit::Zombie, Unit::Initiate, Unit::Skeleton, Unit::Serpent, Unit::Warg, Unit::Ghost, Unit::Wight, Unit::Haunt, Unit::Shrieker, Unit::Spectre, Unit::Rat, Unit::Sorcerer, Unit::Witch, Unit::Vampire, Unit::Mummy, Unit::Lich, Unit::Void, Unit::Cerberus, Unit::Wraith, Unit::Horror, Unit::Banshee, Unit::Elemental, Unit::Harpy, Unit::Shadowlord, Unit::BasicNecromancer, Unit::ArcaneNecromancer, Unit::RangedNecromancer, Unit::MountedNecromancer, Unit::DeadlyNecromancer, Unit::BattleNecromancer, Unit::ZombieNecromancer, Unit::ManaNecromancer, Unit::TerrainNecromancer].iter() {
            let stats = unit.stats();
            if stats.cost > 0 && stats.cost <= money && stats.spawn {
                if stats.cost < cheapest_cost {
                    cheapest_cost = stats.cost;
                    cheapest_unit = Some(*unit);
                }
            }
        }
        if let Some(unit) = cheapest_unit {
            // Find a random empty location
            let mut empty_locs = Vec::new();
            for y in 0..10 {
                for x in 0..10 {
                    let loc = crate::core::Loc::new(x, y);
                    if board.get_piece(&loc).is_none() {
                        empty_locs.push(loc);
                    }
                }
            }
            if !empty_locs.is_empty() {
                let idx = rng.gen_range(0..empty_locs.len());
                let spawn_loc = empty_locs[idx];
                actions.push(BoardAction::Spawn {
                    spawn_loc,
                    unit,
                });
            }
        }
        // Dummy fallback if no spawn actions generated
        if actions.is_empty() {
            println!("[DEBUG] No spawn actions found, generating dummy null move");
            actions.push(BoardAction::EndPhase);
        }
        actions
    }
}