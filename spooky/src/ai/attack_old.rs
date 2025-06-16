use z3::{ast::{Ast, Bool, Int}, Config, Context, SatResult, Solver};
use bumpalo::{Bump, collections::Vec};

use crate::core::{Board, SideArray};
use super::{
    eval::Eval, 
    mcts::{MCTSNode, NodeStats, Stage}, 
    search::SearchArgs, spawn::{SpawnNode, SpawnNodeRef}
};

use std::cell::RefCell;

use rand::prelude::*;

mod combat;
use combat::{CombatPair, CombatGraph};

mod constraints;
use constraints::{add_constraints, Variables};

pub struct AttackNode<'a> {
    pub stats: NodeStats,
    pub board: Board,
    pub delta_points: SideArray<i32>,
    pub delta_money: SideArray<i32>,
    pub combat_graph: CombatGraph,
    pub vars: Variables<'a>,
    pub solver: Solver<'a>,
    pub parent: Option<SpawnNodeRef<'a>>,
    pub children: Vec<'a, SpawnNodeRef<'a>>,
}

pub type AttackNodeRef<'a> = &'a RefCell<AttackNode<'a>>;

impl<'a> AttackNode<'a> {
    pub fn new(board: Board, arena: &'a Bump) -> Self {
        let cfg = Config::new();
        let ctx = arena.alloc(Context::new(&cfg));
        let mut solver = Solver::new(ctx);

        let combat_graph = board.combat_graph();
        let vars = add_constraints(ctx, &mut solver, &combat_graph);

        Self { 
            stats: NodeStats::new(),
            solver,
            board,
            delta_points: SideArray::new(0, 0),
            delta_money: SideArray::new(0, 0),
            combat_graph,
            vars,
            children: Vec::new_in(arena), 
            parent: None,
        }
    }
}

impl<'a> MCTSNode<'a> for AttackNode<'a> {
    type Child = SpawnNode<'a>;
    type Etc = ();

    fn stats(&self) -> &NodeStats {
        &self.stats
    }

    fn children(&self) -> &Vec<'a, SpawnNodeRef<'a>> {
        &self.children
    }

    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, _etc: ()) -> Option<usize> {
        let solver = &self.solver;

        let result = solver.check();

        match result {
            SatResult::Sat => {},
            SatResult::Unsat => return None,
            SatResult::Unknown => panic!("Z3 returned unknown"),
        }

        let model = solver.get_model().unwrap();

        

        todo!()
    }

    fn update(&mut self, eval: &Eval) {
        self.stats.update(eval);

        if let Some(parent) = &self.parent {
            parent.borrow_mut().update(eval);
        }
    }
}
