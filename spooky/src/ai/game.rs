use std::{
    cell::RefCell,
    collections::HashMap,
};

use crate::core::{GameConfig, GameState, Side, SideArray, Turn, action::BoardAction};

use super::{
    mcts::{MCTSNode, NodeStats},
    attack::{AttackNode, AttackNodeRef},
    blotto::blotto,
    eval::Eval,
    general::{GeneralNode, GeneralNodeRef},
    search::SearchArgs,
};

use rand::prelude::*;

use bumpalo::{Bump, collections::Vec};
use std::vec::Vec as StdVec;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct NodeIndex {
    pub general_index: usize,
    pub attack_indices: StdVec<usize>,
    pub spawn_indices: StdVec<usize>,
}

pub struct GameNode<'a> {
    pub stats: NodeStats,
    pub state: GameState,
    pub index_map: HashMap<NodeIndex, usize>,
    pub general_node: GeneralNodeRef<'a>,
    pub board_nodes: Vec<'a, AttackNodeRef<'a>>,
    pub children: StdVec<GameNodeRef<'a>>,
}

pub type GameNodeRef<'a> = &'a RefCell<GameNode<'a>>;

impl <'a> GameNode<'a> {
    pub fn new(parent: &GameNode<'a>, general_next: GeneralNodeRef<'a>, boards_next: Vec<'a, AttackNodeRef<'a>>, args: &SearchArgs<'a>) -> Self {
        let mut board_points = parent.state.board_points.clone();
        let mut money = parent.state.money.clone();

        for b in &boards_next {
            let b = b.borrow();
            board_points[Side::S0] += b.delta_points[Side::S0];
            board_points[Side::S1] += b.delta_points[Side::S1];

            money[Side::S0] += b.delta_money[Side::S0];
            money[Side::S1] += b.delta_money[Side::S1];
        }

        let mut g = general_next.borrow_mut();

        money[Side::S0] += g.delta_money[Side::S0];
        money[Side::S1] += g.delta_money[Side::S1];

        let state = GameState {
            tech_state: g.tech_state.clone(),
            side_to_move: !parent.state.side_to_move,
            boards: boards_next.iter().map(|b| b.borrow().board.clone()).collect(),
            board_points,
            money,
        };

        let eval = Eval::static_eval(args.config, &state).flip();

        g.update(&eval);

        boards_next.iter().for_each(|b| b.borrow_mut().update(&eval));

        Self {
            stats: NodeStats {
                visits: 1,
                eval,
            },
            state,
            index_map: HashMap::new(),
            general_node: general_next,
            board_nodes: boards_next,
            children: StdVec::new(),
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
            state.boards.into_iter()
                .map(|b| arena.alloc(RefCell::new({
                    let mut node = AttackNode::new(b, arena);
                    node.update(&eval);

                    node
                })) as AttackNodeRef),
            arena);

        Self {
            stats: NodeStats {
                visits: 1,
                eval,
            },
            state: state_clone,
            index_map: HashMap::new(),
            general_node,
            board_nodes,
            children: StdVec::new(),
        }
    }

    pub fn explore(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng) -> Eval {
        let (is_new, explore_index) = self.poll(args, rng, ());
        let mut next_node = self.children[explore_index].borrow_mut();

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

    pub fn best_turn(&self) -> Turn {
        todo!()
        // let best_child = self.children.iter().max_by_key(|c| c.stats.visits).unwrap();
        // best_child.best_turn()
    }
// Helper to check if a node is a dummy node (for future reference)
impl<'a> GameNode<'a> {
    pub fn is_dummy(&self) -> bool {
        self.children.is_empty() && self.index_map.is_empty()
    }
}


impl<'a> MCTSNode<'a> for GameNode<'a> {
    type Child = GameNode<'a>;
    type Etc = ();

    fn stats(&self) -> &NodeStats {
        &self.stats
    }

    fn children(&self) -> &StdVec<&'a RefCell<Self::Child>> {
        // Convert from StdVec<GameNodeRef<'a>> to StdVec<&'a RefCell<GameNode<'a>>>
        // This is a bit of a hack, but it should work
        unsafe { std::mem::transmute(&self.children) }
    }

    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, _etc: ()) -> (bool, usize) {
        // Step 1: Get general node child (tech decisions)
        let money = self.state.money[self.state.side_to_move];
        let (general_index, general_next) = self.general_node.borrow_mut().get_child(args, rng, money);

        // Step 2: Apply attack moves to each board
        let mut attack_indices = StdVec::new();
        let mut boards_after_attack = StdVec::new();

        for (i, board_node) in self.board_nodes.iter().enumerate() {
            let (attack_index, _) = board_node.borrow_mut().get_child(args, rng, ());
            attack_indices.push(attack_index);

            // Get the updated board state from the attack node
            let board_after_attack = board_node.borrow().board.clone();
            boards_after_attack.push(board_after_attack);
        }

        // Step 3: Apply blotto allocation
        let blotto_allocations = super::blotto::BlottoStage::allocate(&self.state, self.state.side_to_move);

        // Step 4: Apply spawn actions to each board
        let mut spawn_indices = StdVec::new();
        let mut final_boards = StdVec::new();

        for (board_idx, board) in boards_after_attack.iter().enumerate() {
            let blotto_allocation = blotto_allocations.get(board_idx).copied().unwrap_or(0);
            let spawn_actions = super::spawn::SpawnStage::generate_spawns(board, self.state.side_to_move, blotto_allocation, rng);

            // Debug: print number of spawn actions
            println!("[DEBUG] Board {}: {} spawn actions generated", board_idx, spawn_actions.len());

            // Apply spawn actions to create final board state
            let mut final_board = board.clone();
            for action in &spawn_actions {
                if let BoardAction::Spawn { spawn_loc, unit } = action {
                    let piece = crate::core::board::Piece {
                        loc: *spawn_loc,
                        side: self.state.side_to_move,
                        unit: *unit,
                        modifiers: crate::core::board::Modifiers::default(),
                        state: crate::core::board::PieceState::default().into(),
                    };
                    final_board.add_piece(piece);
                }
            }

            final_boards.push(final_board);
            spawn_indices.push(0); // Simplified for now
        }

        // Create the next game state
        let mut next_state = self.state.clone();
        next_state.side_to_move = !self.state.side_to_move;
        next_state.boards = final_boards;

        // Update tech state and money based on general node
        let general_node = general_next.borrow();
        next_state.tech_state = general_node.tech_state.clone();
        next_state.money[Side::S0] += general_node.delta_money[Side::S0];
        next_state.money[Side::S1] += general_node.delta_money[Side::S1];

        // Debug: print if any children will be created
        if self.children.len() > 100 {
            println!("[DEBUG] Warning: children vector is very large: {}", self.children.len());
        }

        // Create the next game node
        let next_node = args.arena.alloc(RefCell::new(
            GameNode::from_state(next_state, args.config, args.arena)
        ));

        let child_index = self.children.len();
        self.children.push(next_node);

        // Store the index mapping
        let next_index = NodeIndex {
            general_index,
            attack_indices,
            spawn_indices,
        };
        self.index_map.insert(next_index, child_index);

        println!("[DEBUG] make_child: created child at index {} (total children: {})", child_index, self.children.len());

        // Ensure at least one child exists
        if self.children.is_empty() {
            // Create a dummy node
            let mut dummy_state = self.state.clone();
            // Optionally mark this as dummy in state (e.g., set a field or add a log)
            println!("[DEBUG] make_child: creating dummy node for GameNode");
            let dummy_node = args.arena.alloc(RefCell::new(GameNode {
                stats: NodeStats::new(),
                state: dummy_state,
                index_map: HashMap::new(),
                general_node: self.general_node.clone(),
                board_nodes: self.board_nodes.clone(),
                children: StdVec::new(),
            }));
            self.children.push(dummy_node);
            return (true, self.children.len() - 1);
        }

        (true, child_index)
    }

    fn update(&mut self, eval: &Eval) {
        let n = self.stats.visits as f32;

        self.stats.update(eval);

        self.general_node.borrow_mut().update(eval);
        self.board_nodes.iter_mut()
            .for_each(|node| node.borrow_mut().update(eval));
    }
}