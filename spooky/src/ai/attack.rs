use z3::{Config, Context, Solver, ast::{Bool, Int, Ast}};
use bumpalo::{Bump, collections::Vec};

use crate::core::{Board, SideArray};
use super::{
    search::SearchArgs,
    mcts::{Stage, NodeStats},
    spawn::{SpawnNode, SpawnNodeRef},
};

use std::cell::RefCell;

pub struct AttackNode<'a> {
    pub stats: NodeStats,
    pub board: Board,
    pub delta_points: SideArray<i32>,
    pub delta_money: SideArray<i32>,
    pub solver: Solver<'a>,
    pub children: Vec<'a, SpawnNodeRef<'a>>,
}

pub type AttackNodeRef<'a> = &'a RefCell<AttackNode<'a>>;

impl<'a> AttackNode<'a> {
    pub fn new(board: Board, arena: &'a Bump) -> Self {
        let cfg = Config::new();
        let ctx = arena.alloc(Context::new(&cfg));
        let solver = Solver::new(ctx);

        Self { 
            stats: NodeStats::new(),
            solver,
            board,
            delta_points: SideArray::new(0, 0),
            delta_money: SideArray::new(0, 0),
            children: Vec::new_in(arena), 
        }
    }

    pub fn explore(&mut self, args: &SearchArgs<'a>) -> &'a mut SpawnNode<'a> {
        todo!()
    }
}

