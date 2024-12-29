//! Strategic decision making across multiple boards

use std::{
    cell::RefCell, 
    collections::HashMap,
    // borrow::Borrow,
    // ops::Borrow,
};

use crate::core::{GameConfig, GameState, Side, SideArray, Turn};

use super::{
    attack::{AttackNode, AttackNodeRef}, 
    blotto::blotto, 
    eval::Eval, 
    general::{GeneralNode, GeneralNodeRef}, 
    search::SearchArgs, 
};

use rand::prelude::*;

use bumpalo::{Bump, collections::Vec};
use std::vec::Vec as StdVec;

#[derive(Debug, Clone, Copy)]
pub enum Stage {
    General,
    Attack,
    Blotto,
    Spawn,
}

pub struct NodeStats {
    pub visits: u32,
    pub eval: Eval,
}

impl NodeStats {
    pub fn new() -> Self {
        Self {
            visits: 0,
            eval: Eval::new(0.0),
        }
    }

    pub fn uct(&self, ln_n: f32, rng: &mut impl Rng) -> f32 {
        const C: f32 = 1.5;
        const EPS: f32 = 0.1;

        let w = self.eval.winprob();
        let n0 = self.visits as f32;
        let r = rng.gen_range(-EPS..EPS);

        w + C * (ln_n / (n0 + 1.0)).sqrt() + r
    }

    pub fn update(&mut self, eval: &Eval) {
        let n = self.visits as f32;

        self.visits += 1;
        self.eval.winprob = (self.eval.winprob * n + eval.winprob) / (n + 1.0);
    }
}

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
    pub children: Vec<'a, GameNodeRef<'a>>,
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

        let eval = Eval::static_eval(args.config, &state);

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
            children: Vec::new_in(args.arena),
        }        
    }

    pub fn from_state(state: GameState, config: &GameConfig, arena: &'a Bump) -> Self {
        let state_clone = state.clone();
        let eval = Eval::static_eval(config, &state);

        let general_node = arena.alloc(RefCell::new({
            let mut node = GeneralNode::new(state.tech_state, SideArray::new(0, 0), arena);
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
            children: Vec::new_in(arena),
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
        let n = self.stats.visits as f32;

        self.stats.update(eval);

        self.general_node.borrow_mut().update(eval);
        self.board_nodes.iter_mut()
            .for_each(|node| node.borrow_mut().update(eval));
    }

    // pub fn poll(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng) -> (bool, usize) {
    //     // let general_next = self.general_node.explore(arena);
    //     // let boards_next = self.board_nodes.iter_mut()
    //     //     .map(|n| n.explore(arena));
    //     let mut best_child_index = 0;
    //     let mut best_uct = f32::NEG_INFINITY;
    //     let mut total_score: f32 = 0.0;
    //     let ln_n = (self.stats.visits as f32).ln();

    //     for (i, child) in self.children.iter().enumerate() {
    //         let uct = child.stats.uct(ln_n, rng);
    //         if uct > best_uct {
    //             best_child_index = i;
    //             best_uct = uct;
    //         }

    //         let score = child.winprob();

    //         total_score += score;
    //     }

    //     let avg_score = total_score / self.children.len().min(1) as f32;

    //     let phantom_stats = NodeStats {
    //         visits: 1 + self.children.len() as u32,
    //         eval: Eval::new(avg_score),
    //     };

    //     let phantom_uct = phantom_stats.uct(ln_n, rng);

    //     if best_uct > phantom_uct {
    //         (false, best_child_index)
    //     } else {
    //         self.make_child(args, rng)
    //     }
    // }

    // pub fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng) -> (bool, usize) {
    //     let (general_index, general_next) = self.general_node.borrow_mut().poll(args, rng);
    //     let (attack_indices, boards_next) = self.board_nodes.iter()
    //             .map(|node| node.borrow_mut().poll(args, rng))
    //             .unzip::<_, _, StdVec<_>, StdVec<_>>();

    //     let blotto = blotto(self, &general_next.borrow(), &boards_next, rng);

    //     let (spawn_indices, boards_next) = boards_next
    //         .into_iter()
    //         .zip(blotto)
    //         .map(|(node, blotto)| node.poll(args, blotto, rng))
    //         .unzip::<_, _, StdVec<_>, StdVec<_>>();

    //     let next_index = NodeIndex {
    //         general_index,
    //         attack_indices,
    //         spawn_indices,
    //     };

    //     if let Some(&index) = self.index_map.get(&next_index) {
    //         return (false, index);
    //     }

    //     let boards_next = Vec::from_iter_in(boards_next, args.arena);

    //     let new_child = GameNode::new(self, &general_next, boards_next, args);
    //     let new_child_index = self.children.len();

    //     self.children.push(new_child);
    //     self.index_map.insert(next_index, new_child_index);

    //     (true, new_child_index)
    // }

    

    pub fn best_turn(&self) -> Turn {
        todo!()
        // let best_child = self.children.iter().max_by_key(|c| c.stats.visits).unwrap();
        // best_child.best_turn()
    }
}

pub trait MCTSNode<'a> {
    type Child: MCTSNode<'a>;
    type Etc;

    fn stats(&self) -> &NodeStats;
    fn children(&self) -> &Vec<'a, &'a RefCell<Self::Child>>;
    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, etc: Self::Etc) -> (bool, usize);

    fn poll(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, etc: Self::Etc) -> (bool, usize)
        where Self: 'a {

        let mut best_child_index = 0;
        let mut best_uct = f32::NEG_INFINITY;
        let mut total_score: f32 = 0.0;
        let ln_n = (self.stats().visits as f32).ln();

        for (i, child) in self.children().iter().enumerate() {
            let child = child.borrow();
            let stats = child.stats();

            let uct = stats.uct(ln_n, rng);
            if uct > best_uct {
                best_child_index = i;
                best_uct = uct;
            }

            let score = stats.eval.winprob();

            total_score += score;
        }

        let avg_score = total_score / self.children().len().min(1) as f32;

        let phantom_stats = NodeStats {
            visits: 1 + self.children().len() as u32,
            eval: Eval::new(avg_score),
        };

        let phantom_uct = phantom_stats.uct(ln_n, rng);

        if best_uct > phantom_uct {
            (false, best_child_index)
        } else {
            self.make_child(args, rng, etc)
        }
    }

    fn eval(&self) -> Eval {
        self.stats().eval
    }

    fn get_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, etc: Self::Etc) -> (usize, &'a RefCell<Self::Child>) 
        where Self: 'a {

        let (_, index) = self.poll(args, rng, etc);

        (index, self.children()[index])
    }

    fn update(&mut self, eval: &Eval);

    // fn get_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng) -> (usize, Self::Child) 
    // where Self: 'a {
    //     let (_, child_index) = self.poll(args, rng);

    //     (child_index, self.children()[child_index])
    // }
}

impl<'a> MCTSNode<'a> for GameNode<'a> {
    type Child = GameNode<'a>;
    type Etc = ();

    fn stats(&self) -> &NodeStats {
        &self.stats
    }

    fn children(&self) -> &Vec<'a, GameNodeRef<'a>> {
        &self.children
    }

    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, etc: Self::Etc) -> (bool, usize) {
        let (general_index, general_next) = self.general_node.borrow_mut().get_child(args, rng, etc);
        let (attack_indices, boards_next) = self.board_nodes.iter()
                .map(|node| node.borrow_mut().get_child(args, rng, etc))
                .unzip::<_, _, StdVec<_>, StdVec<_>>();

        let blotto = blotto(self, &general_next.borrow(), &boards_next, rng);

        let (spawn_indices, boards_next) = boards_next
            .into_iter()
            .zip(blotto)
            .map(|(node, blotto)| node.borrow_mut().get_child(args, rng, blotto))
            .unzip::<_, _, StdVec<_>, StdVec<_>>();

        let next_index = NodeIndex {
            general_index,
            attack_indices,
            spawn_indices,
        };

        if let Some(&index) = self.index_map.get(&next_index) {
            return (false, index);
        }

        let boards_next = Vec::from_iter_in(boards_next, args.arena);

        let new_child = args.arena.alloc(RefCell::new(
            GameNode::new(self, &general_next, boards_next, args)
        ));
        let new_child_index = self.children.len();

        self.children.push(new_child);
        self.index_map.insert(next_index, new_child_index);

        (true, new_child_index)
    }

    fn update(&mut self, eval: &Eval) {
        let n = self.stats.visits as f32;

        self.stats.update(eval);

        self.general_node.borrow_mut().update(eval);
        self.board_nodes.iter_mut()
            .for_each(|node| node.borrow_mut().update(eval));
    }
}