use super::{eval::Eval, mcts::{MCTSNode, NodeStats}, search::SearchArgs};
use crate::core::{Board, SideArray, Side, units::Unit, action::BoardAction};

use std::cell::RefCell;

use rand::prelude::*;

use std::vec::Vec;


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