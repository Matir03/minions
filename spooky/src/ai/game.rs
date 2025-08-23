use std::{cell::RefCell, collections::HashMap, ops::AddAssign};

use crate::ai::captain::node::{BoardNode, BoardNodeRef, BoardNodeState};
use crate::ai::eval::Eval;
use crate::ai::general::{GeneralNode, GeneralNodeRef, GeneralNodeState};
use crate::ai::mcts::{MCTSEdge, MCTSNode, NodeState, NodeStats};
use crate::core::board::actions::BoardTurn;
use crate::core::spells::Spell;
use crate::core::tech::{TechAssignment, Techline, SPELL_COST};
use crate::core::{
    board::Board, tech::TechState, GameConfig, GameState, GameTurn, Side, SideArray,
};
use hashbag::HashBag;

use crate::ai::blotto::distribute_money;
use crate::ai::search::SearchArgs;

use rand::prelude::*;

use bumpalo::{collections::Vec as BumpVec, Bump};
use std::vec::Vec as StdVec;

pub struct GameNodeState<'a> {
    pub game_state: GameState<'a>,
    pub general_node: GeneralNodeRef<'a>,
    pub board_nodes: BumpVec<'a, BoardNodeRef<'a>>,
}

impl<'a> std::fmt::Debug for GameNodeState<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameNodeState")
            .field("game_state", &self.game_state)
            .finish()
    }
}

impl<'a> PartialEq for GameNodeState<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.game_state == other.game_state
    }
}

impl<'a> Eq for GameNodeState<'a> {}

impl<'a> GameNodeState<'a> {
    pub fn from_game_state(game_state: GameState<'a>, arena: &'a Bump) -> Self {
        let general_mcts_node = arena.alloc(RefCell::new(MCTSNode::new(
            GeneralNodeState::new(
                game_state.side_to_move,
                game_state.tech_state.clone(),
                SideArray::new(0, 0),
            ),
            game_state.side_to_move,
            arena,
        )));
        let mut board_mcts_nodes = BumpVec::new_in(arena);

        for board_state in &game_state.boards {
            let board_mcts_node: BoardNodeRef<'a> = arena.alloc(RefCell::new(MCTSNode::new(
                BoardNodeState::new(board_state.clone(), game_state.side_to_move),
                game_state.side_to_move,
                arena,
            )));
            board_mcts_nodes.push(board_mcts_node);
        }

        Self {
            game_state,
            general_node: general_mcts_node,
            board_nodes: board_mcts_nodes,
        }
    }
}

impl<'a> NodeState<GameTurn> for GameNodeState<'a> {
    type Args = (SearchArgs<'a>, &'a Bump);

    fn propose_move(&self, rng: &mut impl Rng, args_tuple: &Self::Args) -> (GameTurn, Self) {
        let (search_args, arena) = args_tuple;

        let current_side = self.game_state.side_to_move;

        let total_money_current_side = self.game_state.money[current_side];

        // First, poll the general node to decide on tech.
        let mut general_mcts_node_borrowed = self.general_node.borrow_mut();
        let general_args = (total_money_current_side, search_args.config.clone());
        let (_g_is_new, g_child_idx) =
            general_mcts_node_borrowed.poll(&search_args, rng, general_args);
        let general_turn = general_mcts_node_borrowed.edges[g_child_idx].turn.clone();
        let next_general_state_ref = general_mcts_node_borrowed.edges[g_child_idx].child;
        let next_general_node_state_snapshot = next_general_state_ref.borrow().state.clone();

        let money_for_spells = general_turn.num_spells() * search_args.config.spell_cost();
        let next_tech_state = next_general_node_state_snapshot.tech_state;

        // Now, distribute the remaining money.
        let (money_for_general, money_for_boards) = distribute_money(
            total_money_current_side,
            money_for_spells,
            self.game_state.boards.len(),
        );

        let mut board_turns = StdVec::with_capacity(self.board_nodes.len());
        let mut next_board_states_for_gamestate = StdVec::with_capacity(self.board_nodes.len());
        let mut next_board_mcts_node_refs = BumpVec::new_in(arena);
        let mut total_board_delta_money = SideArray::new(0, 0);
        let mut total_board_delta_points = SideArray::new(0, 0);

        for (i, board_mcts_node_ref) in self.board_nodes.iter().enumerate() {
            let mut board_mcts_node_borrowed = board_mcts_node_ref.borrow_mut();

            let board_money = *money_for_boards.get(i).unwrap();
            let cur_tech_state = self.game_state.tech_state.clone();
            let board_args = (
                board_money,
                cur_tech_state,
                search_args.config,
                i,
                self.game_state.ply,
            );

            let (_b_is_new, b_child_idx) =
                board_mcts_node_borrowed.poll(&search_args, rng, board_args);

            let board_turn = board_mcts_node_borrowed.edges[b_child_idx].turn.clone();
            let next_board_state_ref = board_mcts_node_borrowed.edges[b_child_idx].child;
            let next_board_node_state_snapshot = next_board_state_ref.borrow().state.clone();

            board_turns.push(board_turn);
            next_board_mcts_node_refs.push(next_board_state_ref);

            total_board_delta_money.add_assign(&next_board_node_state_snapshot.delta_money);
            total_board_delta_points.add_assign(&next_board_node_state_snapshot.delta_points);
            next_board_states_for_gamestate.push(next_board_node_state_snapshot.board);
        }

        let mut next_money = self.game_state.money.clone();
        next_money[current_side] += next_general_node_state_snapshot.delta_money[current_side];
        next_money.add_assign(&total_board_delta_money);

        let mut next_board_points = self.game_state.board_points.clone();
        next_board_points.add_assign(&total_board_delta_points);

        let next_winner = [current_side, !current_side]
            .into_iter()
            .find(|&side| next_board_points[side] >= self.game_state.config.points_to_win);

        let next_game_state = GameState {
            config: self.game_state.config,
            tech_state: next_tech_state,
            side_to_move: !current_side,
            boards: next_board_states_for_gamestate,
            board_points: next_board_points,
            money: next_money,
            ply: self.game_state.ply + 1,
            winner: next_winner,
        };

        let num_boards = self.game_state.config.num_boards;
        let game_turn = GameTurn {
            tech_assignment: general_turn,
            board_turns,
            spells: HashBag::new(),
            spell_assignment: vec![Spell::Blank; num_boards],
        };

        let next_game_node_state = GameNodeState {
            game_state: next_game_state,
            general_node: next_general_state_ref,
            board_nodes: next_board_mcts_node_refs,
        };

        (game_turn, next_game_node_state)
    }
}

pub type GameNode<'a> = MCTSNode<'a, GameNodeState<'a>, GameTurn>;
pub type GameNodeRef<'a> = &'a RefCell<GameNode<'a>>;

impl<'a> GameNode<'a> {
    pub fn is_terminal(&self) -> bool {
        self.state.game_state.winner.is_some()
    }
}
