pub mod combat;
pub mod reposition;
pub mod spawn;

/// MCTS Node representing the state of a single board and its turn processing (attack + spawn).
use std::cell::RefCell;
use std::ops::AddAssign;

use rand::prelude::*;

use crate::{
    core::{
        board::{Board, actions::BoardTurn, BoardState},
        game::{GameConfig, GameState},
        map::Map,
        phase::Phase,
        side::{Side, SideArray},
        tech::TechState,
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

/// Represents the state of a single board and its turn processing (attack + spawn).
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

    fn apply_turn_to_board(
        original_board: &Board,
        turn: &BoardTurn,
        side_to_move: Side,
        money: &mut i32,
    ) -> Board {
        let mut new_board = original_board.clone();

        for action in &turn.setup_actions {
            if let Err(e) = new_board.do_setup_action(side_to_move, action.clone()) {
                eprintln!("[BoardNodeState] Error applying setup action: {:?}: {}", action, e);
            }
        }
        for action in &turn.attack_actions {
            if let Err(e) = new_board.do_attack_action(side_to_move, action.clone()) {
                eprintln!("[BoardNodeState] Error applying attack action: {:?}: {}", action, e);
            }
        }
        for action in &turn.spawn_actions {
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
                let board_after_combat = Self::apply_turn_to_board(
                    &self.board,
                    &BoardTurn {
                        setup_actions: vec![],
                        attack_actions: attack_actions.clone(), // TODO: Avoid clone
                        spawn_actions: vec![],
                    },
                    self.side_to_move,
                    &mut money_after_combat,
                );

                let mut spawn_actions = Vec::new();
                if *turn_num != 1 {
                    spawn::generate_heuristic_spawn_actions(
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
                    &BoardTurn {
                        setup_actions: vec![],
                        attack_actions: vec![],
                        spawn_actions: spawn_actions.clone(), // TODO: Avoid clone
                    },
                    self.side_to_move,
                    &mut money_after_spawn,
                );

                let mut proposed_turn_delta_money = SideArray::new(0, 0);
                proposed_turn_delta_money[self.side_to_move] = money_after_spawn - *money_allocation;

                let turn_taken = BoardTurn {
                    setup_actions: vec![],
                    attack_actions,
                    spawn_actions,
                };

                let new_state = Self {
                    board: final_board_state,
                    side_to_move: self.side_to_move,
                    delta_money: proposed_turn_delta_money,
                    delta_points: SideArray::new(0,0), // TODO: Calculate points delta
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

#[cfg(test)]
mod tests {
    use crate::ai::mcts::NodeState;
    use crate::core::{board::Board, game::GameConfig, map::Map, side::Side, tech::TechState, board::actions::AttackAction};
    use super::BoardNodeState;
    use rand::thread_rng;

    #[test]
    fn test_ai_proposes_non_trivial_opening_move() {
        let board = Board::from_fen(Board::START_FEN, Map::BlackenedShores).unwrap();
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