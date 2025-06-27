/// MCTS Node representing the state of a single board and its turn processing (attack + spawn).
use std::{cell::RefCell, fmt};
use std::ops::AddAssign;

use rand::prelude::*;

use crate::core::board::definitions::Phase;
use crate::core::Unit;
use crate::{
    core::{
        board::{
            actions::{AttackAction, BoardTurn, SetupAction, SpawnAction},
            Board,
        },
        game::{GameConfig, GameState},
        map::Map,
        side::{Side, SideArray},
        tech::TechState,
    },
    ai::{eval::Eval, mcts::{MCTSNode, NodeState, NodeStats}},
};

use super::{
    combat::{
        solver::{block_solution, generate_move_from_model},
        stage::CombatStage,
    },
    positioning::PositioningStage,
    spawn::generate_heuristic_spawn_actions,
};

use z3::{Config, Context, Optimize, SatResult};

/// Represents the state of a single board and its turn processing (attack + spawn).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardNodeState<'a> {
    pub board: Board<'a>,
    pub side_to_move: Side,
    pub delta_money: SideArray<i32>,
    pub delta_points: SideArray<i32>,
}

impl<'a> BoardNodeState<'a> {
    pub fn new(board: Board<'a>, side_to_move: Side) -> Self {
        Self {
            board,
            side_to_move,
            delta_money: SideArray::new(0, 0),
            delta_points: SideArray::new(0, 0),
        }
    }
}

pub fn setup_phase(side: Side, board: &Board<'_>) -> SetupAction {
    let saved_unit = board.reinforcements[side].iter()
        .max_by_key(|unit| unit.stats().cost)
        .cloned();

    SetupAction {
        necromancer_choice: Unit::ArcaneNecromancer,
        saved_unit,
    }
}

impl<'a> NodeState<BoardTurn> for BoardNodeState<'a> {
    // Args: (money, tech_state, config, current_board_idx, turn_num)
    type Args = (i32, TechState, &'a GameConfig, usize, i32);

    fn propose_move(&self, rng: &mut impl Rng, args: &Self::Args) -> (BoardTurn, Self) {
        let (money, ref tech_state, _config, _current_board_idx, turn_num) = *args;

        let z3_cfg = Config::new();
        let ctx = Context::new(&z3_cfg);
        let optimizer = Optimize::new(&ctx);

        // --- Setup Stage ---
        let setup_action = if self.board.state.phases().contains(&Phase::Setup) {
            Some(setup_phase(self.side_to_move, &self.board))
        } else {
            None
        };

        // --- Combat Stage ---
        let combat_stage = CombatStage::new(&ctx, &optimizer);
        let combat_solver = combat_stage.add_constraints(&self.board, self.side_to_move);

        // --- Repositioning Stage ---
        let reposition_stage = PositioningStage::new(&ctx, &optimizer);
        reposition_stage.add_constraints(
            &self.board,
            self.side_to_move,
            &combat_solver.graph,
            &combat_solver.variables,
        );

        let combat_result = optimizer.check(&[]);
        let attack_actions = match combat_result {
            SatResult::Sat => generate_move_from_model(
                &optimizer.get_model().unwrap(),
                &combat_solver.graph,
                &combat_solver.variables,
                &self.board,
            ),
            SatResult::Unknown | SatResult::Unsat => 
                panic!("[BoardNodeState] Z3 solver returned {:?}", combat_result),
        };

        let mut new_board = self.board.clone();
        let rebate = new_board.do_attacks(
            self.side_to_move,
            &attack_actions,
        ).expect(&format!(
            "[BoardNodeState] Failed to perform attack phase actions: {:#?}",
            attack_actions,
        ));

        let spawn_actions = if self.board.state.phases().contains(&Phase::Spawn) {
            generate_heuristic_spawn_actions(
                &new_board,
                self.side_to_move,
                tech_state,
                money,
                rng,
            )
        } else {
            Vec::new()
        };

        let money_after_spawn = new_board.do_spawns(
            self.side_to_move,
            money,
            &spawn_actions,
        ).expect(&format!(
            "[BoardNodeState] Failed to perform spawn phase actions: {:#?}",
            spawn_actions,
        ));

        let (income, winner) = new_board.end_turn(self.side_to_move)
            .expect("[BoardNodeState] Failed to end turn");

        let mut delta_money = SideArray::new(0, 0);
        delta_money[self.side_to_move] = money_after_spawn - money + income;
        delta_money[!self.side_to_move] = rebate;

        let turn_taken = BoardTurn {
            setup_action: None,
            attack_actions,
            spawn_actions,
        };

        let mut delta_points = SideArray::new(0, 0);
        if let Some(winner) = winner {
            delta_points[winner] = 1;
        }

        let new_state = Self {
            board: new_board,
            side_to_move: !self.side_to_move,
            delta_money,
            delta_points,
        };

        (turn_taken, new_state)
    }
}

pub type BoardNode<'a> = MCTSNode<'a, BoardNodeState<'a>, BoardTurn>;
pub type BoardNodeRef<'a> = &'a RefCell<BoardNode<'a>>;

#[cfg(test)]
mod tests {
    use crate::ai::mcts::NodeState;
    use crate::core::{
        board::Board,
        board::actions::AttackAction,
        game::GameConfig,
        map::{Map, MapSpec},
        side::Side,
        tech::{Tech, TechAssignment, TechState, Techline},
    };
    use rand::thread_rng;

    use super::BoardNodeState;

    fn new_all_unlocked_tech_state() -> TechState {
        let mut tech_state = TechState::new();
        let techline = Techline::default();
        let mut acquire_indices = Vec::new();

        // Collect all 0-based indices for unit techs
        for i in 0..techline.len() {
            if let Tech::UnitTech(unit) = techline[i] {
                if unit.stats().cost > 0 {
                    acquire_indices.push(i);
                }
            }
        }

        // To acquire techs, we must first advance the unlock_index past them.
        let assignment_s0 = TechAssignment::new(techline.len(), acquire_indices.clone());
        let assignment_s1 = TechAssignment::new(techline.len(), acquire_indices);

        tech_state
            .assign_techs(assignment_s0, Side::Yellow, &techline)
            .unwrap();
        // Don't unwrap, as some techs may have been acquired by S0 already
        let _ = tech_state.assign_techs(assignment_s1, Side::Blue, &techline);

        tech_state
    }

    #[test]
    fn test_ai_proposes_non_trivial_opening_move() {
        let map = Map::BlackenedShores;
        let board = Board::from_fen(Board::START_FEN, &map).unwrap();
        let mut rng = thread_rng();
        let tech_state = new_all_unlocked_tech_state();
        let config = GameConfig::default();
        let node_state = BoardNodeState::new(board, Side::Yellow);

        let (turn, _new_state) = node_state.propose_move(&mut rng, &(12, tech_state, &config, 0, 0));

        assert!(!turn.spawn_actions.is_empty());
    }
}
