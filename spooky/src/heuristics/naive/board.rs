use std::collections::{HashMap, HashSet};

use crate::{
    core::{
        board::{
            actions::{BoardTurn, BoardActions, SpawnAction, AttackAction, SetupAction}, 
            Board, definitions::Phase, BitboardOps
        },
        map::Map,
        Side, SideArray, loc::Loc, tech::TechState, units::Unit, 
        convert::FromIndex, Tech, ToIndex,
    },
    heuristics::traits::{BoardHeuristic, LocalHeuristic},
    ai::captain::{
        positioning::SatPositioningSystem,
        combat::{
            constraints::{generate_move_from_sat_model, ConstraintManager},
            generation::CombatGenerationSystem,
        },
    },
};

use super::techline::NaiveShared;
use rand::{distributions::WeightedIndex, prelude::*};
use bumpalo::Bump;
use z3::{Config, Context};

#[derive(Debug)]
pub struct NaiveBoardHeuristic {
    map: Map,
}

// Accumulator represents the current board evaluation state
#[derive(Debug, Clone)]
pub struct BoardAcc {
    pub board: Board<'static>, // This lifetime issue needs to be resolved
    pub side_to_move: Side,
    pub delta_money: SideArray<i32>,
    pub delta_points: SideArray<i32>,
}

// Removed BoardEnc - not needed in simplified system

// Preprocessed data for turn computation including complete board generation state
#[derive(Debug, Clone)]
pub struct BoardPre {
    pub setup_action: Option<SetupAction>,
    pub resign_available: bool,
}

impl<'a> LocalHeuristic<&'a Map, Board<'a>, BoardTurn, NaiveShared> for NaiveBoardHeuristic {
    type Acc = BoardAcc;
    type Pre = BoardPre;

    fn new(config: &'a Map) -> Self {
        Self {
            map: *config,
        }
    }

    fn compute_acc(&self, state: &Board<'a>) -> Self::Acc {
        let static_board = unsafe { std::mem::transmute::<Board<'a>, Board<'static>>(state.clone()) };
        
        BoardAcc {
            board: static_board,
            side_to_move: Side::Yellow,
            delta_money: SideArray::new(0, 0),
            delta_points: SideArray::new(0, 0),
        }
    }

    fn update_acc(&self, acc: &Self::Acc, turn: &BoardTurn) -> Self::Acc {
        let mut new_acc = acc.clone();
        new_acc.side_to_move = !acc.side_to_move;
        
        // Apply turn effects to delta money/points
        match turn {
            BoardTurn::Actions(actions) => {
                // Update deltas based on actions (simplified)
            }
            BoardTurn::Resign => {
                // Resignation penalty
                new_acc.delta_points[!acc.side_to_move] = 1; // Opponent gets a point
            }
        }
        
        new_acc
    }

    fn compute_pre(&self, state: &Board<'a>, _acc: &Self::Acc) -> Self::Pre {
        // Setup phase detection and processing
        let setup_action = if state.state.phases().contains(&Phase::Setup) {
            Some(Self::setup_phase(Side::Yellow, state)) // Use Yellow as default, will be corrected
        } else {
            None
        };
        
        BoardPre {
            setup_action,
            resign_available: true,
        }
    }

    fn compute_turn(&self, blotto: i32, shared: &NaiveShared, pre: &Self::Pre) -> BoardTurn {
        if blotto <= 0 {
            return BoardTurn::Resign;
        }
        
        // For now, return a simple action
        // In the full implementation, this would use the complete BoardChildGen logic
        if let Some(setup_action) = &pre.setup_action {
            BoardTurn::Actions(BoardActions {
                setup_action: Some(setup_action.clone()),
                attack_actions: vec![], // Simplified
                spawn_actions: vec![], // Simplified
            })
        } else {
            BoardTurn::Resign
        }
    }
}

// Complete heuristic implementations copied from AI system
impl NaiveBoardHeuristic {
    /// Setup phase heuristic copied from ai::captain::node::setup_phase
    pub fn setup_phase(side: Side, board: &Board<'_>) -> SetupAction {
        let saved_unit = board.reinforcements[side]
            .iter()
            .max_by_key(|unit| (unit.stats().cost, unit.to_index().unwrap()))
            .cloned();

        SetupAction {
            necromancer_choice: Unit::BasicNecromancer,
            saved_unit,
        }
    }
    
    /// Complete spawn heuristics copied from ai::captain::spawn
    pub fn generate_heuristic_spawn_actions(
        board: &Board,
        side: Side,
        tech_state: &TechState,
        mut money: i32,
        rng: &mut impl Rng,
    ) -> Vec<SpawnAction> {
        let mut actions = Vec::new();

        // Part 1: Decide what to buy and create `Buy` actions
        let units_to_buy = Self::purchase_heuristic(side, tech_state, money, rng);
        for &unit in &units_to_buy {
            money -= unit.stats().cost;
            actions.push(SpawnAction::Buy { unit });
        }

        // Part 2: Decide what to spawn from all available reinforcements
        let mut all_units_to_potentially_spawn = board.reinforcements[side].clone();
        for &unit in &units_to_buy {
            all_units_to_potentially_spawn.insert(unit);
        }

        // Greedily spawn the most expensive units
        let mut sorted_units = all_units_to_potentially_spawn
            .iter()
            .copied()
            .collect::<Vec<_>>();
        sorted_units.sort_by_key(|u| -(u.stats().cost as i32));

        // Get spawn locations based on the current board state
        let mut all_spawn_locs = board.bitboards.get_spawn_locs(side, true);
        let mut land_spawn_locs = board.bitboards.get_spawn_locs(side, false);

        for unit in sorted_units {
            if unit.stats().flying {
                if let Some(loc) = all_spawn_locs.pop() {
                    actions.push(SpawnAction::Spawn {
                        spawn_loc: loc,
                        unit,
                    });
                    land_spawn_locs.set(loc, false);
                } else {
                    break;
                }
            } else {
                if let Some(loc) = land_spawn_locs.pop() {
                    actions.push(SpawnAction::Spawn {
                        spawn_loc: loc,
                        unit,
                    });
                    all_spawn_locs.set(loc, false);
                }
            }
        }

        actions
    }

    const WEIGHT_FACTOR: f64 = 1.2;
    
    /// Purchase heuristic copied from ai::captain::spawn::purchase_heuristic
    fn purchase_heuristic(
        side: Side,
        tech_state: &TechState,
        mut money: i32,
        rng: &mut impl Rng,
    ) -> Vec<Unit> {
        let available_units: Vec<_> = tech_state.acquired_techs[side]
            .iter()
            .filter_map(|tech| {
                if let Tech::UnitTech(unit) = tech {
                    Some(*unit)
                } else {
                    None
                }
            })
            .chain(Unit::BASIC_UNITS.into_iter())
            .collect();

        let mut units_with_weights = available_units
            .iter()
            .map(|u| (u, Self::WEIGHT_FACTOR.powi(u.to_index().unwrap() as i32)))
            .collect::<Vec<_>>();

        let mut units_to_buy = Vec::new();

        loop {
            units_with_weights = units_with_weights
                .into_iter()
                .filter(|(u, _)| money >= u.stats().cost)
                .collect();

            if units_with_weights.is_empty() {
                break;
            }

            let weights: Vec<f64> = units_with_weights.iter().map(|(_, w)| *w).collect();
            let distr = WeightedIndex::new(&weights).unwrap();
            let idx = distr.sample(rng);
            let unit = units_with_weights[idx].0;

            units_to_buy.push(*unit);
            money -= unit.stats().cost;
        }

        units_to_buy
    }
    
    /// Process turn end copied from ai::captain::node::process_turn_end
    pub fn process_turn_end<'a>(
        mut board: Board<'a>,
        side_to_move: Side,
        money: i32,
        money_after_spawn: i32,
        rebate: i32,
    ) -> BoardAcc {
        let (income, winner) = board
            .end_turn(side_to_move)
            .expect("[BoardAcc] Failed to end turn");

        let mut delta_points = SideArray::new(0, 0);
        if let Some(winner) = winner {
            delta_points[winner] = 1;
        }

        let mut delta_money = SideArray::new(0, 0);
        delta_money[side_to_move] = money_after_spawn - money + income;
        delta_money[!side_to_move] = rebate;

        let static_board = unsafe { std::mem::transmute::<Board<'a>, Board<'static>>(board) };

        BoardAcc {
            board: static_board,
            side_to_move: !side_to_move,
            delta_money,
            delta_points,
        }
    }
    
    /// Complete board turn generation that would use the full BoardChildGen logic
    pub fn generate_complete_turn(
        board: &Board,
        side: Side,
        money: i32,
        tech_state: &TechState,
        rng: &mut impl Rng,
        arena: &Bump,
    ) -> BoardTurn {
        // This would contain the complete logic from BoardChildGen::propose_turn
        // For now, simplified implementation
        
        let mut actions = BoardActions {
            setup_action: None,
            attack_actions: vec![],
            spawn_actions: vec![],
        };
        
        // Setup phase
        if board.state.phases().contains(&Phase::Setup) {
            actions.setup_action = Some(Self::setup_phase(side, board));
        }
        
        // Spawn phase
        if board.state.phases().contains(&Phase::Spawn) && board.winner.is_none() {
            actions.spawn_actions = Self::generate_heuristic_spawn_actions(
                board, side, tech_state, money, rng
            );
        }
        
        BoardTurn::Actions(actions)
    }
}

impl<'a> BoardHeuristic<'a, NaiveShared> for NaiveBoardHeuristic {}