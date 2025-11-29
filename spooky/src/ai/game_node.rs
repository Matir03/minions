use std::marker::PhantomData;
use std::{cell::RefCell, collections::HashMap, ops::AddAssign};

use crate::ai::captain::board_node::{BoardNode, BoardNodeArgs, BoardNodeRef, BoardNodeState};
use crate::ai::eval::Eval;
use crate::ai::general_node::{GeneralNode, GeneralNodeArgs, GeneralNodeRef, GeneralNodeState};
use crate::ai::mcts::{ChildGen, MCTSEdge, MCTSNode, NodeStats};
use crate::core::board::actions::BoardTurn;
use crate::core::spells::Spell;
use crate::core::tech::{TechAssignment, Techline, SPELL_COST};
use crate::core::Blotto;
use crate::core::{
    board::Board, tech::TechState, GameConfig, GameState, GameTurn, Side, SideArray,
};
use crate::heuristics::{self, BlottoGen, Heuristic};
use derive_where::derive_where;
use hashbag::HashBag;

use rand::prelude::*;

use bumpalo::{collections::Vec as BumpVec, Bump};
use std::vec::Vec as StdVec;

#[derive_where(Clone)]
pub struct GameNodeState<'a, H: Heuristic<'a>> {
    pub heuristic_state: H::CombinedEnc,
    pub game_state: GameState<'a>,
    pub general_node: GeneralNodeRef<'a, H::CombinedEnc, H>,
    pub board_nodes: BumpVec<'a, BoardNodeRef<'a, H::CombinedEnc, H>>,
}

// Manual PartialEq/Eq implementation required because we compare RefCell pointers,
// not their contents. The heuristic_state is intentionally skipped.
impl<'a, H: Heuristic<'a>> PartialEq for GameNodeState<'a, H> {
    fn eq(&self, other: &Self) -> bool {
        self.general_node.as_ptr() == other.general_node.as_ptr()
            && self.board_nodes.iter().zip(other.board_nodes.iter()).all(
                |(board_node, other_board_node)| board_node.as_ptr() == other_board_node.as_ptr(),
            )
    }
}

impl<'a, H: Heuristic<'a>> Eq for GameNodeState<'a, H> {}

impl<'a, H: Heuristic<'a>> GameNodeState<'a, H> {
    pub fn from_game_state(game_state: GameState<'a>, arena: &'a Bump, heuristic: &H) -> Self {
        let general_mcts_node = MCTSNode::new(
            GeneralNodeState::new(
                game_state.side_to_move,
                game_state.tech_state.clone(),
                SideArray::new(0, 0),
                heuristic,
            ),
            game_state.side_to_move,
            arena,
        );
        let general_enc = &general_mcts_node.state.heuristic_state;

        let mut board_nodes_in_arena = BumpVec::new_in(arena);

        for board_state in &game_state.boards {
            let board_mcts_node = MCTSNode::new(
                BoardNodeState::new(board_state.clone(), game_state.side_to_move, heuristic),
                game_state.side_to_move,
                arena,
            );
            let board_node_in_arena: BoardNodeRef<'a, H::CombinedEnc, H> =
                arena.alloc(RefCell::new(board_mcts_node));
            board_nodes_in_arena.push(board_node_in_arena);
        }

        // TODO: don't use clone here
        let board_encs = board_nodes_in_arena
            .iter()
            .map(|b| b.borrow().state.heuristic_state.clone())
            .collect::<StdVec<_>>();

        let board_encs_ref = board_encs.iter().collect::<StdVec<_>>();

        let heuristic_state = heuristic.compute_combined(&game_state, general_enc, &board_encs_ref);
        let general_node_in_arena = arena.alloc(RefCell::new(general_mcts_node));

        Self {
            heuristic_state,
            game_state,
            general_node: general_node_in_arena,
            board_nodes: board_nodes_in_arena,
        }
    }
}

// --------------------------
// Child generator for Game
// --------------------------

pub struct GameChildGen<'a, H: Heuristic<'a>> {
    blotto_gen: H::BlottoGen,
}

#[derive_where(Clone)]
pub struct GameChildGenArgs<'a, H: Heuristic<'a>> {
    pub arena: &'a Bump,
    pub heuristic: &'a H,
}

impl<'a, H: Heuristic<'a>> ChildGen<GameNodeState<'a, H>, GameTurn> for GameChildGen<'a, H> {
    type Args = GameChildGenArgs<'a, H>;

    fn new(state: &GameNodeState<'a, H>, _rng: &mut impl Rng, args: Self::Args) -> Self {
        let GameChildGenArgs { arena, heuristic } = args;
        GameChildGen {
            blotto_gen: heuristic.compute_blottos(&state.heuristic_state),
        }
    }

    fn propose_turn(
        &mut self,
        state: &GameNodeState<'a, H>,
        rng: &mut impl Rng,
        args: Self::Args,
    ) -> Option<(GameTurn, GameNodeState<'a, H>)> {
        let GameChildGenArgs { arena, heuristic } = args;
        let config = state.game_state.config;

        let current_side = state.game_state.side_to_move;

        let total_money_current_side = state.game_state.money[current_side];

        // First, poll the general node to decide on tech.
        let mut general_mcts_node_borrowed = state.general_node.borrow_mut();

        let pre_turn = heuristic.compute_general_pre_turn(
            rng,
            &state.heuristic_state,
            &general_mcts_node_borrowed.state.heuristic_state,
        );

        let general_args = GeneralNodeArgs {
            money: total_money_current_side,
            config,
            heuristic,
            pre_turn,
            _phantom: PhantomData,
        };
        let (_g_is_new, g_child_idx) =
            general_mcts_node_borrowed.poll(arena, heuristic, rng, general_args);
        let general_turn = general_mcts_node_borrowed.edges[g_child_idx].turn.clone();
        let next_general_state_ref = general_mcts_node_borrowed.edges[g_child_idx].child;
        let next_general_node_state_snapshot = &next_general_state_ref.borrow().state;

        let money_for_spells = general_turn.num_spells() * config.spell_cost();
        let next_tech_state = &next_general_node_state_snapshot.tech_state;

        // Now, distribute the remaining money.
        let Blotto { money_for_boards } = self.blotto_gen.blotto(money_for_spells);

        let mut board_turns = StdVec::with_capacity(state.board_nodes.len());
        let mut next_board_states_for_gamestate = StdVec::with_capacity(state.board_nodes.len());
        let mut next_board_mcts_node_refs = BumpVec::new_in(arena);
        let mut total_board_delta_money = SideArray::new(0, 0);
        let mut total_board_delta_points = SideArray::new(0, 0);

        let cur_tech_state = &state.game_state.tech_state.clone();

        for (i, board_mcts_node_ref) in state.board_nodes.iter().enumerate() {
            let mut board_mcts_node_borrowed = board_mcts_node_ref.borrow_mut();

            let board_money = money_for_boards[i];
            let board_args = BoardNodeArgs {
                money: board_money,
                tech_state: cur_tech_state.clone(),
                config,
                arena,
                heuristic,
                _c: PhantomData,
            };

            let (_b_is_new, b_child_idx) =
                board_mcts_node_borrowed.poll(arena, heuristic, rng, board_args);

            let board_turn = board_mcts_node_borrowed.edges[b_child_idx].turn.clone();
            let next_board_state_ref = board_mcts_node_borrowed.edges[b_child_idx].child;
            let next_board_node_state_snapshot = next_board_state_ref.borrow().state.clone();

            board_turns.push(board_turn);
            next_board_mcts_node_refs.push(next_board_state_ref);

            total_board_delta_money.add_assign(&next_board_node_state_snapshot.delta_money);
            total_board_delta_points.add_assign(&next_board_node_state_snapshot.delta_points);
            next_board_states_for_gamestate.push(next_board_node_state_snapshot.board);
        }

        let mut next_money = state.game_state.money.clone();
        next_money[current_side] += next_general_node_state_snapshot.delta_money[current_side];
        next_money.add_assign(&total_board_delta_money);

        let mut next_board_points = state.game_state.board_points.clone();
        next_board_points.add_assign(&total_board_delta_points);

        let next_winner = [current_side, !current_side]
            .into_iter()
            .find(|&side| next_board_points[side] >= config.points_to_win);

        let next_game_state = GameState {
            config,
            game_id: state.game_state.game_id.clone(),
            tech_state: next_tech_state.clone(),
            side_to_move: !current_side,
            boards: next_board_states_for_gamestate,
            board_points: next_board_points,
            money: next_money,
            ply: state.game_state.ply + 1,
            winner: next_winner,
        };

        let num_boards = config.num_boards;
        let game_turn = GameTurn {
            tech_assignment: general_turn,
            board_turns,
            spells: HashBag::new(),
            spell_assignment: vec![Spell::Blank; num_boards],
        };

        let general_enc = &next_general_node_state_snapshot.heuristic_state;
        let board_encs = next_board_mcts_node_refs
            .iter()
            .map(|b| b.borrow().state.heuristic_state.clone())
            .collect::<StdVec<_>>();
        let board_encs_ref = board_encs.iter().collect::<StdVec<_>>();
        let heuristic_state =
            heuristic.compute_combined(&next_game_state, &general_enc, &board_encs_ref);

        let next_game_node_state = GameNodeState {
            game_state: next_game_state,
            general_node: next_general_state_ref,
            board_nodes: next_board_mcts_node_refs,
            heuristic_state,
        };

        Some((game_turn, next_game_node_state))
    }
}

pub type GameNode<'a, H> = MCTSNode<'a, GameNodeState<'a, H>, GameTurn, GameChildGen<'a, H>>;
pub type GameNodeRef<'a, H> = &'a RefCell<GameNode<'a, H>>;

impl<'a, H: Heuristic<'a>> GameNode<'a, H> {
    pub fn is_terminal(&self) -> bool {
        self.state.game_state.winner.is_some()
    }
}
