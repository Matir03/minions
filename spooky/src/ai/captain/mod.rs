pub mod combat;
pub mod reposition;
pub mod spawn;

/// MCTS Node representing the state of a single board and its turn processing (attack + spawn).
use std::cell::RefCell;
use std::vec::Vec as StdVec;
use std::ops::AddAssign;

use bumpalo::{Bump, collections::Vec};
use rand::prelude::*;

use crate::{
    core::{
        action::BoardAction,
        board::Board,
        game::GameConfig,
        side::Side,
        tech::TechState,
        map::Map,
        SideArray,
    },
    ai::{eval::Eval, mcts::{MCTSNode, NodeState, NodeStats}},
};

use self::{
    combat::{
        solver::{block_solution, generate_move_from_model},
        CombatStage,
    },
    reposition::RepositioningStage,
    spawn::generate_heuristic_spawn_actions,
};

use z3::{Config, Context, Optimize, SatResult};

/// Represents the actions taken during a single board's turn phase.
#[derive(Clone, Debug)]
pub struct BoardTurn {
    pub attack_actions: StdVec<BoardAction>,
    pub spawn_actions: StdVec<BoardAction>,
}

#[derive(Clone, Debug)]
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
        let mut turn_delta_money = SideArray::new(0, 0);
        let mut turn_delta_points = SideArray::new(0, 0);

        for action in actions {
            if let Err(e) = new_board.do_action(
                action.clone(),
                &mut turn_delta_money,
                &mut turn_delta_points,
                tech_state,
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

    fn propose_move(&self, _rng: &mut impl Rng, args: &<Self as NodeState<BoardTurn>>::Args) -> (BoardTurn, Self) {
        let (money_allocation, tech_state, config, current_board_idx) = args;

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


        // --- Solve and generate actions ---
        match optimizer.check(&[]) {
            SatResult::Sat => {
                let model = optimizer.get_model().unwrap();
                let combat_actions =
                    generate_move_from_model(&model, &combat_solver.graph, &combat_solver.variables);


                let mut proposed_turn_delta_money = SideArray::new(0, 0);
                let mut proposed_turn_delta_points = SideArray::new(0, 0);

                // Apply combat actions and generate spawn actions based on the new state
                let (board_after_combat, combat_delta_money, combat_delta_points) =
                    Self::apply_actions_to_board(
                        &self.board,
                        &combat_actions,
                        self.side_to_move,
                        config,
                        &tech_state,
                        *current_board_idx,
                    );
                proposed_turn_delta_money.add_assign(&combat_delta_money);
                proposed_turn_delta_points.add_assign(&combat_delta_points);

                // Heuristically determine and apply spawn actions
                let spawn_actions = generate_heuristic_spawn_actions(
                    &board_after_combat,
                    self.side_to_move,
                    &tech_state,
                    *money_allocation - combat_delta_money[self.side_to_move],
                );

                let (final_board_state, spawn_delta_money, spawn_delta_points) =
                    Self::apply_actions_to_board(
                        &board_after_combat,
                        &spawn_actions,
                        self.side_to_move,
                        config,
                        &tech_state,
                        *current_board_idx,
                    );
                proposed_turn_delta_money.add_assign(&spawn_delta_money);
                proposed_turn_delta_points.add_assign(&spawn_delta_points);

                let turn_taken = BoardTurn {
                    attack_actions: combat_actions,
                    spawn_actions,
                };

                let new_state = Self {
                    board: final_board_state,
                    side_to_move: self.side_to_move,
                    delta_money: proposed_turn_delta_money,
                    delta_points: proposed_turn_delta_points,
                };

                (turn_taken, new_state)
            }
            SatResult::Unsat | SatResult::Unknown => {
                panic!("there should always be a legal solution")
            }
        }
    }
}

pub type BoardNode<'a> = MCTSNode<'a, BoardNodeState, BoardTurn>;
pub type BoardNodeRef<'a> = &'a RefCell<BoardNode<'a>>;