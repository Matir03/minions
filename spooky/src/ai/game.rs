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
            arena,
        )));
        let mut board_mcts_nodes = BumpVec::new_in(arena);

        for board_state in &game_state.boards {
            let board_mcts_node: BoardNodeRef<'a> = arena.alloc(RefCell::new(MCTSNode::new(
                BoardNodeState::new(board_state.clone(), game_state.side_to_move),
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

        let next_game_state = GameState {
            config: self.game_state.config,
            tech_state: next_tech_state,
            side_to_move: !current_side,
            boards: next_board_states_for_gamestate,
            board_points: next_board_points,
            money: next_money,
            ply: self.game_state.ply + 1,
            winner: self.game_state.winner,
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

// impl<'a> GameNode<'a> {
//     pub fn construct_best_turn(&self) -> GameTurn {
//         let tech_assignment = self.state.general_node.borrow().best_turn();

//         let board_turns = self.state.board_nodes
//             .iter()
//             .map(|board_node_ref| board_node_ref.borrow().best_turn())
//             .collect();

//         let num_boards = self.state.game_state.config.num_boards;
//         GameTurn {
//             tech_assignment,
//             board_turns,
//             spells: HashBag::new(),
//             spell_assignment: vec![Spell::Blank; num_boards],
//         }
//     }
// }
/*
pub struct GameTree<'a> {
    pub mcts_node_ref: &'a RefCell<MCTSNode<'a, GameNodeState<'a>, GameTurn>>,
}

impl <'a> GameTree<'a> {
    pub fn from_state(initial_game_state: GameState, search_args: SearchArgs<'a>, arena: &'a Bump) -> Self {
        let initial_general_node_state = GeneralNodeState::new(
            initial_game_state.side_to_move,
            initial_game_state.tech_state.clone(),
            SideArray::new(0,0)
        );
        let general_mcts_node = arena.alloc(RefCell::new(MCTSNode::new(initial_general_node_state, arena)));

        let mut initial_board_mcts_nodes = BumpVec::new_in(arena);
        for board_state in &initial_game_state.boards {
            let initial_board_node_state = BoardNodeState::new(
                board_state.clone(),
                initial_game_state.side_to_move
            );
            let board_mcts_node = arena.alloc(RefCell::new(MCTSNode::new(initial_board_node_state, arena)));
            initial_board_mcts_nodes.push(board_mcts_node);
        }

        let initial_game_node_state = GameNodeState {
            game_state: initial_game_state.clone(),
            general_mcts_node,
            board_mcts_nodes: initial_board_mcts_nodes,
        };

        let root_mcts_node = MCTSNode::new(initial_game_node_state, arena);
        let root_mcts_node_ref = arena.alloc(RefCell::new(root_mcts_node));
        Self {
            mcts_node_ref: root_mcts_node_ref,
        }
    }

    pub fn explore(&mut self, search_args: SearchArgs<'a>, rng: &mut impl Rng, arena: &'a Bump) -> Eval {
        let gns_args = (search_args, arena);

        let mut path: BumpVec<&'a RefCell<MCTSNode<'a, GameNodeState, GameTurn>>> = BumpVec::new_in(arena);
        let mut current_mcts_node_ref = self.mcts_node_ref;
        path.push(current_mcts_node_ref);

        let mut current_mcts_node = current_mcts_node_ref.borrow_mut();

        while !(*current_mcts_node).is_leaf() {
            let (_, best_child_idx) = current_mcts_node.poll(&search_args, rng, gns_args);
            let next_node_ref = current_mcts_node.edges[best_child_idx].child;
            path.push(next_node_ref);
            let temp_next_node_ref = current_mcts_node.edges[best_child_idx].child;
            drop(current_mcts_node);
            current_mcts_node_ref = temp_next_node_ref;
            current_mcts_node = current_mcts_node_ref.borrow_mut();
        }

        let eval_from_new_leaf = if (*current_mcts_node).stats.visits > 0 || (*current_mcts_node).is_terminal(&search_args.config.techline) {
            if (*current_mcts_node).is_terminal(&search_args.config.techline) {
                 Eval::static_eval(search_args.config, &(*current_mcts_node).state.game_state)
            } else {
                (*current_mcts_node).make_child(&search_args, rng, gns_args);
                let new_child_node = (*current_mcts_node).edges.last().unwrap().child.borrow();
                Eval::static_eval(search_args.config, &new_child_node.state.game_state)
            }
        } else {
            Eval::static_eval(search_args.config, &(*current_mcts_node).state.game_state)
        };

        for node_ref_in_path in path.iter().rev() {
            let mut node_in_path = node_ref_in_path.borrow_mut();
            let perspective_eval = if node_in_path.state.game_state.side_to_move != eval_from_new_leaf.side.unwrap_or(node_in_path.state.game_state.side_to_move) {
                eval_from_new_leaf.flip()
            } else {
                eval_from_new_leaf
            };
            node_in_path.update(&perspective_eval);
        }

        eval_from_new_leaf
    }

    pub fn best_turn(&self) -> GameTurn {
        let mcts_node = self.mcts_node_ref.borrow();
        if mcts_node.edges.is_empty() {
            panic!("best_turn() called on GameNode with no explored children.");
        }
        let best_edge = mcts_node.edges.iter().max_by_key(|edge| edge.child.borrow().stats.visits)
            .expect("No children to select best turn from");
        best_edge.turn.clone()
    }
}

impl<'a> MCTSNode<'a, GameNodeState<'a>, GameTurn> {
    pub fn is_terminal(&self, techline: &Techline) -> bool {
        self.state.game_state.is_terminal(techline)
    }
}
*/
