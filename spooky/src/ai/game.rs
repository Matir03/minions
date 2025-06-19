use std::{
    cell::RefCell,
    collections::HashMap,
};

use crate::core::{GameConfig, GameState, Side, SideArray, GameTurn, board::Board, tech::TechState};
use crate::ai::mcts::{MCTSNode, NodeStats};
use crate::ai::general::{GeneralNode, GeneralNodeRef};
use crate::ai::board_node::{BoardNode, BoardNodeRef};
use crate::ai::eval::Eval;
use crate::ai::search::SearchArgs;
use crate::ai::blotto::distribute_money;

use rand::prelude::*;

use bumpalo::{Bump, collections::Vec};
use std::vec::Vec as StdVec;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct NodeIndex {
    pub general_child_idx: usize, 
    // We might not need to store board_child_indices here if GameNode's make_child
    // deterministically creates one new state per board based on BoardNode's make_child.
    // If BoardNode itself had multiple strategic children to choose from that GameNode needed to track,
    // then we'd need something like: pub board_node_child_indices: StdVec<usize>,
}

pub struct GameEdge<'a> {
    pub turn: GameTurn,
    pub child: GameNodeRef<'a>,
}

pub struct GameNode<'a> {
    pub stats: NodeStats,
    pub state: GameState,
    // pub index_map: HashMap<NodeIndex, usize>,
    pub general_node: GeneralNodeRef<'a>,
    pub board_nodes: Vec<'a, BoardNodeRef<'a>>, 
    pub edges: Vec<'a, GameEdge<'a>>,
}

pub type GameNodeRef<'a> = &'a RefCell<GameNode<'a>>;

impl <'a> GameNode<'a> {
    pub fn from_nodes(parent: &GameNode<'a>, general_node: GeneralNodeRef<'a>, board_nodes: Vec<'a, BoardNodeRef<'a>>, args: &SearchArgs<'a>) -> Self {
        let mut board_points = parent.state.board_points.clone();
        let mut money = parent.state.money.clone();

        for b in &board_nodes {
            let b = b.borrow();
            board_points[Side::S0] += b.delta_points[Side::S0];
            board_points[Side::S1] += b.delta_points[Side::S1];

            money[Side::S0] += b.delta_money[Side::S0];
            money[Side::S1] += b.delta_money[Side::S1];
        }

        let mut g = general_node.borrow_mut();

        money[Side::S0] += g.delta_money[Side::S0];
        money[Side::S1] += g.delta_money[Side::S1];

        let state = GameState {
            tech_state: g.tech_state.clone(),
            side_to_move: !parent.state.side_to_move,
            boards: board_nodes.iter().map(|b_node_ref| b_node_ref.borrow().board.clone()).collect(), 
            board_points,
            money,
        };

        let eval = Eval::static_eval(args.config, &state).flip();

        g.update(&eval);

        board_nodes.iter().for_each(|b| b.borrow_mut().update(&eval));

        Self {
            stats: NodeStats {
                visits: 1,
                eval,
            },
            state,
            // index_map: HashMap::new(),
            general_node,
            board_nodes,
            edges: Vec::new_in(args.arena),
        }
    }

    pub fn from_state(state: GameState, config: &GameConfig, arena: &'a Bump) -> Self {
        let state_clone = state.clone();
        let eval = Eval::static_eval(config, &state).flip();

        let general_node = arena.alloc(RefCell::new({
            let mut node = GeneralNode::new(state.side_to_move, state.tech_state, SideArray::new(0, 0), arena);
            node.update(&eval);

            node
        }));

        let board_nodes = Vec::from_iter_in(
            state.boards.iter() 
                .map(|b_state| arena.alloc(RefCell::new({
                    let mut node = BoardNode::new(b_state.clone(), state.side_to_move, arena);
                    node.stats.eval = eval.clone(); 
                    node
                })) as BoardNodeRef<'a>),
            arena);

        Self {
            stats: NodeStats {
                visits: 1,
                eval,
            },
            state: state_clone,
            // index_map: HashMap::new(),
            general_node,
            board_nodes,
            edges: Vec::new_in(arena),
        }
    }

    pub fn explore(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng) -> Eval {
        let (is_new, explore_index) = self.poll(args, rng, ());
        let mut next_node = self.edges[explore_index].child.borrow_mut();

        let eval = if is_new {
            next_node.eval()
        } else {
            next_node.explore(args, rng)
        };

        self.update(&eval);

        eval
    }

    pub fn update(&mut self, eval: &Eval) {
        let eval = &eval.flip();
        let n = self.stats.visits as f32;

        self.stats.update(eval);

        self.general_node.borrow_mut().update(eval);
        self.board_nodes.iter_mut()
            .for_each(|node| node.borrow_mut().update(eval));
    }

    pub fn eval(&self) -> Eval {
        self.stats.eval.flip()
    }

    pub fn best_turn(&self) -> GameTurn {
        self.edges.iter()
            .max_by_key(|edge| 
                edge.child.borrow().stats.visits
            )
            .unwrap()
            .turn
            .clone()
    }


}

impl<'a> MCTSNode<'a> for GameNode<'a> {
    type Args = ();

    fn stats(&self) -> &NodeStats {
        &self.stats
    }

    fn children(&self) -> &Vec<'a, &'a RefCell<Self>> {
        &self.edges.child
    }

    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, _etc: ()) -> (bool, usize) {
        let (money_for_general, money_for_boards) = distribute_money(self.state.money[self.state.side_to_move], self.state.boards.len(), rng);

        let (general_child_idx, next_general_node_ref) = 
            self.general_node.borrow_mut().get_child(args, rng, money_for_general);
        let general_delta_money = next_general_node_ref.borrow().delta_money.clone(); // And provides delta money

        // Get the TechState from the chosen general node child. This will be cloned for each board node.
        let tech_state_from_general = next_general_node_ref.borrow().tech_state.clone();

        // 3. Board Nodes: Get next board states for each board
        let mut next_board_states = StdVec::with_capacity(self.board_nodes.len());
        let mut next_board_node_refs_for_new_game_node = Vec::from_iter_in(
            self.board_nodes.iter().enumerate().map(|(i, board_node_ref)| {
                let board_money = money_for_boards.get(i).copied().unwrap_or(0);
                // BoardNode's Etc is now (money_allocation, TechState) - pass a clone.
                let (_board_child_idx, next_board_node_child_ref) = 
                    board_node_ref.borrow_mut().get_child(args, rng, (board_money, tech_state_from_general.clone()));
                
                // The new GameNode will be constructed with these evolved BoardNodes.
                // So, we collect the refs to these *children* of the current GameNode's BoardNodes.
                next_board_states.push(next_board_node_child_ref.borrow().board.clone());
                next_board_node_child_ref
            }),
            args.arena
        );

        // 4. Construct the next GameState
        let mut next_game_state = GameState {
            tech_state: next_general_node_ref.borrow().tech_state.clone(), // Tech state from the chosen general node child
            side_to_move: !self.state.side_to_move,
            boards: next_board_states, // Board states from chosen board_node children
            board_points: self.state.board_points.clone(), // Points are updated by Board::do_action, which BoardNode calls
            money: self.state.money.clone(), // Base money for next turn
        };

        // Apply delta money from general node spending
        next_game_state.money[Side::S0] += general_delta_money[Side::S0];
        next_game_state.money[Side::S1] += general_delta_money[Side::S1];

        // Apply delta money and points from child board nodes
        for board_node_ref in next_board_node_refs_for_new_game_node.iter() {
            let board_node_child = board_node_ref.borrow(); // This is the child BoardNode for the *next* state
            
            next_game_state.money[Side::S0] += board_node_child.delta_money[Side::S0];
            next_game_state.money[Side::S1] += board_node_child.delta_money[Side::S1];
            
            next_game_state.board_points[Side::S0] += board_node_child.delta_points[Side::S0];
            next_game_state.board_points[Side::S1] += board_node_child.delta_points[Side::S1];
        }

        // 5. Create the new GameNode child
        // The `GameNode::new` constructor takes parent, general_next_ref, boards_next_refs.
        // We need to ensure the `boards_next_refs` are the `BoardNodeRef`s that correspond to the `next_board_states`.
        // The `next_board_node_refs_for_new_game_node` are the children resulting from `board_node_ref.borrow_mut().get_child(...)`.

        let new_child_game_node = args.arena.alloc(RefCell::new(GameNode::from_nodes(
            self, // Parent is the current GameNode
            next_general_node_ref, // The child selected from current general_node
            next_board_node_refs_for_new_game_node, // The children selected from current board_nodes
            args,
        )));

        let child_idx = self.children.len();
        self.children.push(new_child_game_node);

        // Update index map
        let node_index = NodeIndex { general_child_idx }; // Simplified NodeIndex
        self.index_map.insert(node_index, child_idx);

        (true, child_idx)
    }

    fn update(&mut self, eval: &Eval) {
        let n = self.stats.visits as f32;

        self.stats.update(eval);

        self.general_node.borrow_mut().update(eval);
        self.board_nodes.iter_mut()
            .for_each(|node| node.borrow_mut().update(eval));
    }
}