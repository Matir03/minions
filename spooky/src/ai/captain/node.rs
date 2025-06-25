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

    fn apply_turn_to_board(
        original_board: &Board,
        setup_actions: &[SetupAction],
        attack_actions: &[AttackAction],
        spawn_actions: &[SpawnAction],
        side_to_move: Side,
        money: &mut i32,
        rebates: &mut SideArray<i32>,
    ) -> Board {
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

impl NodeState<BoardTurn> for BoardNodeState {
    type Args = (i32, TechState, GameConfig, usize, i32); // Added usize for current_board_idx and i32 for turn_num

    fn propose_move(&self, _rng: &mut impl Rng, args: &<Self as NodeState<BoardTurn>>::Args) -> (BoardTurn, Self) {
        let (money_allocation, tech_state, _config, _current_board_idx, turn_num) = args;

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
                let attack_actions =
                    generate_move_from_model(&model, &combat_solver.graph, &combat_solver.variables);

                let mut money_after_combat = *money_allocation;
                let mut rebates = SideArray::new(0, 0);
                let board_after_combat = Self::apply_turn_to_board(
                    &self.board,
                    &[], // setup_actions
                    &attack_actions,
                    &[], // spawn_actions
                    self.side_to_move,
                    &mut money_after_combat,
                    &mut rebates,
                );

                let mut spawn_actions = Vec::new();
                if *turn_num != 1 {
                    generate_heuristic_spawn_actions(
                        &board_after_combat,
                        self.side_to_move,
                        &tech_state,
                        &mut money_after_combat,
                        &mut spawn_actions,
                    );
                }

                let mut money_after_spawn = money_after_combat;
                let final_board_state = Self::apply_turn_to_board(
                    &board_after_combat,
                    &[], // setup_actions
                    &[], // attack_actions
                    &spawn_actions,
                    self.side_to_move,
                    &mut money_after_spawn,
                    &mut rebates,
                );

                let mut proposed_turn_delta_money = SideArray::new(0, 0);
                proposed_turn_delta_money[self.side_to_move] = money_after_spawn - *money_allocation;
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
            SatResult::Unsat | SatResult::Unknown => {
                panic!("Z3 solver returned unsat or unknown")
            }
        }
    }
}

pub type BoardNode<'a> = MCTSNode<'a, BoardNodeState, BoardTurn>;
pub type BoardNodeRef<'a> = &'a RefCell<BoardNode<'a>>;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::ai::mcts::NodeState;
    use crate::core::{board::Board, game::GameConfig, map::Map, side::Side, tech::TechState, board::actions::AttackAction};
    use super::BoardNodeState;
    use rand::thread_rng;

    #[test]
    fn test_ai_proposes_non_trivial_opening_move() {
        let board = Board::from_fen(Board::START_FEN, Arc::new(Map::BlackenedShores)).unwrap();
        let mut rng = thread_rng();
        let tech_state = TechState::new();
        let config = GameConfig::default();
        let node_state = BoardNodeState::new(board, Side::S0);

        let (turn, _new_state) = node_state.propose_move(&mut rng, &(12, tech_state, config, 0, 0));

        let has_move = turn.attack_actions.iter().any(|a|
            matches!(a, AttackAction::Move {..} | AttackAction::MoveCyclic {..} | AttackAction::Attack {..})
        );

        assert!(has_move, "AI should propose a move or attack on the opening turn. Got: {:?}", turn.attack_actions);
    }
}
