use bumpalo::{Bump, collections::Vec};
use std::cell::RefCell;
use std::vec::Vec as StdVec;

use crate::core::{
    Side,
    side::SideArray,
    GameConfig,
    tech::{TechAssignment, TechState},
};

use rand::{distributions::Bernoulli, prelude::*};

use super::{
    mcts::{NodeStats, MCTSNode},
    eval::Eval,
    search::SearchArgs,
};

pub struct GeneralNode<'a> {
    pub stats: NodeStats,
    pub side: Side,
    pub tech_state: TechState,
    pub delta_money: SideArray<i32>,
    pub children: Vec<'a, GeneralNodeRef<'a>>,
    pub assignments: Vec<'a, TechAssignment>,
}

pub type GeneralNodeRef<'a> = &'a RefCell<GeneralNode<'a>>;

impl <'a> GeneralNode<'a> {
    pub fn new(side: Side, tech_state: TechState, delta_money: SideArray<i32>, arena: &'a Bump) -> Self {
        Self {
            stats: NodeStats::new(),
            side,
            tech_state,
            delta_money,
            children: Vec::new_in(arena),
            assignments: Vec::new_in(arena),
        }
    }

    pub fn best_assignment(&self) -> TechAssignment {
        let best_index = self.children.iter().enumerate()
            .max_by_key(|(_, c)| c.borrow().stats.visits)
            .map(|(i, _)| i)
            .unwrap();

        self.assignments[best_index].clone()
    }
}

impl<'a> MCTSNode<'a> for GeneralNode<'a> {
    type Child = GeneralNode<'a>;
    type Etc = i32;

    fn stats(&self) -> &NodeStats {
        &self.stats
    }

    fn children(&self) -> &StdVec<&'a RefCell<Self::Child>> {
        unsafe { std::mem::transmute(&self.children) }
    }

    fn make_child(&mut self, args: &SearchArgs<'a>, rng: &mut impl Rng, money: i32) -> (bool, usize) {
        const P: f64 = 0.8;

        let side = self.side;
        let techline = &args.config.techline;
        let spell_cost = args.config.spell_cost();

        let assignment = if self.children.len() == 0 && self.tech_state.unlock_index[self.side] < techline.len() {
            TechAssignment::new(1, vec![])
        } else {
            let mut acquire = vec![];
            let mut advance = 0;

            let max_spells = (money / spell_cost) as usize;
            let cur_index = self.tech_state.unlock_index[self.side];
            let max_reach = cur_index + max_spells - 1;

            let dist = Bernoulli::new(P).unwrap();

            for i in (0..max_reach).rev() {
                let can_tech = self.tech_state.acquirable(i);

                if !can_tech {
                    continue;
                }

                if dist.sample(rng) {
                    advance = advance.max(i - cur_index + 1);
                    acquire.push(i);

                    let mut spells_left = max_spells - advance - 1;

                    for j in (0..i).rev() {
                        if spells_left == 0 {
                            break;
                        }

                        let can_tech = self.tech_state.acquirable(j);

                        if !can_tech {
                            continue;
                        }

                        if dist.sample(rng) {
                            acquire.push(j);
                            spells_left -= 1;
                        }
                    }

                    break;
                }
            }


            TechAssignment::new(advance, acquire)
        };

        let num_spells = assignment.num_spells();
        let mut delta_money = SideArray::new(0, 0);
        delta_money[side] = num_spells * spell_cost;

        let mut new_tech_state = self.tech_state.clone();
        new_tech_state.assign_techs(assignment.clone(), side, &techline).unwrap();

        let new_child = args.arena.alloc(RefCell::new(
            GeneralNode::new(self.side, new_tech_state, delta_money, args.arena)
        ));

        self.children.push(new_child);
        self.assignments.push(assignment);

        (true, self.children.len() - 1)
    }

    fn update(&mut self, eval: &Eval) {
        self.stats.update(eval);
    }
}