/// MCTS Node representing the state of a single board and its turn processing (attack + spawn).
use std::cell::RefCell;
use std::ops::AddAssign;

use rand::prelude::*;

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
    reposition::RepositioningStage,
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

    fn apply_turn_to_board<'b>(
        original_board: &Board<'b>,
        setup_actions: &[SetupAction],
        attack_actions: &[AttackAction],
        spawn_actions: &[SpawnAction],
        side_to_move: Side,
        money: &mut i32,
        rebates: &mut SideArray<i32>,
    ) -> Board<'b> {
        let mut new_board = original_board.clone();

        for action in setup_actions {
            if let Err(e) = new_board.do_setup_action(side_to_move, action.clone()) {
                eprintln!("[BoardNodeState] Error applying setup action: {:?}: {}", action, e);
            }
        }
        for action in attack_actions {
            match new_board.do_attack_action(side_to_move, action.clone()) {
                Ok(Some((side, amount))) => {
                    rebates[side] += amount;
                }
                Ok(None) => (),
                Err(e) => {
                    eprintln!("[BoardNodeState] Error applying attack action: {:?}: {}", action, e);
                }
            }
        }
        for action in spawn_actions {
            if let Err(e) = new_board.do_spawn_action(side_to_move, money, action.clone()) {
                eprintln!("[BoardNodeState] Error applying spawn action: {:?}: {}", action, e);
            }
        }
        new_board
    }
}

impl<'a> NodeState<BoardTurn> for BoardNodeState<'a> {
    type Args = (i32, TechState, &'a GameConfig, usize, i32); // Added usize for current_board_idx and i32 for turn_num

    fn propose_move(&self, _rng: &mut impl Rng, args: &<Self as NodeState<BoardTurn>>::Args) -> (BoardTurn, Self) {
        let (money_allocation, ref tech_state, _config, _current_board_idx, turn_num) = *args;

        let z3_cfg = Config::new();
        let ctx = Context::new(&z3_cfg);
        let optimizer = Optimize::new(&ctx);

        // --- Combat Stage ---
        let combat_stage = CombatStage::new(&ctx, &optimizer);
        let combat_solver = combat_stage.add_constraints(&self.board, self.side_to_move);

        // --- Repositioning Stage ---
        let reposition_stage = RepositioningStage::new(&ctx, &optimizer);
        reposition_stage.add_constraints(
            &self.board,
            self.side_to_move,
            &combat_solver.graph,
            &combat_solver.variables,
        );

        let combat_result = optimizer.check(&[]);
        let attack_actions = match combat_result {
            SatResult::Sat => generate_move_from_model(&optimizer.get_model().unwrap(), &combat_solver.graph, &combat_solver.variables),
            SatResult::Unsat => Vec::new(),
            SatResult::Unknown => panic!("Z3 solver returned unknown"),
        };

        let mut money_after_attack = money_allocation;
        let mut rebates = SideArray::new(0, 0);
        let board_after_attack = BoardNodeState::apply_turn_to_board(
            &self.board,
            &[],
            &attack_actions,
            &[],
            self.side_to_move,
            &mut money_after_attack,
            &mut rebates,
        );

        let mut spawn_actions = Vec::new();
        generate_heuristic_spawn_actions(
            &board_after_attack,
            self.side_to_move,
            tech_state,
            &mut money_after_attack,
            &mut spawn_actions,
        );

        let mut money_after_spawn = money_after_attack;
        let final_board_state = BoardNodeState::apply_turn_to_board(
            &board_after_attack,
            &[],
            &[],
            &spawn_actions,
            self.side_to_move,
            &mut money_after_spawn,
            &mut rebates,
        );

        let mut proposed_turn_delta_money = SideArray::new(0, 0);
        proposed_turn_delta_money[self.side_to_move] = money_after_spawn - money_allocation;
        proposed_turn_delta_money += rebates;

        let turn_taken = BoardTurn {
            setup_actions: vec![],
            attack_actions,
            spawn_actions,
        };

        let mut delta_points = SideArray::new(0, 0);
        if let Some(winner) = final_board_state.winner {
            delta_points[winner] = 1;
        }

        let new_state = Self {
            board: final_board_state,
            side_to_move: self.side_to_move,
            delta_money: proposed_turn_delta_money,
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
            .assign_techs(assignment_s0, Side::S0, &techline)
            .unwrap();
        // Don't unwrap, as some techs may have been acquired by S0 already
        let _ = tech_state.assign_techs(assignment_s1, Side::S1, &techline);

        tech_state
    }

    #[test]
    fn test_ai_proposes_non_trivial_opening_move() {
        let map = Map::BlackenedShores;
        let board = Board::from_fen(Board::START_FEN, &map).unwrap();
        let mut rng = thread_rng();
        let tech_state = new_all_unlocked_tech_state();
        let config = GameConfig::default();
        let node_state = BoardNodeState::new(board, Side::S0);

        let (turn, _new_state) = node_state.propose_move(&mut rng, &(12, tech_state, &config, 0, 0));

        assert!(!turn.spawn_actions.is_empty());
    }
}
