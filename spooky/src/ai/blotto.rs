//! Resource allocation across boards

use rand::prelude::*;

use super::{general::GeneralNode, game::GameNode, spawn::{SpawnNode, SpawnNodeRef}};

type Blotto = Vec<i32>;

pub fn blotto(node: &GameNode, general_next: &GeneralNode, boards_next: &Vec<SpawnNodeRef>, rng: &mut impl Rng) -> Blotto {
    let side = node.state.side_to_move;
    let money = node.state.money[side] + general_next.delta_money[side];
    let n = node.state.boards.len();

    let mut blotto = vec![0; n];

    for dollar in 0..money {
        let board = rng.gen_range(0..=n);
        if board < n {
            blotto[board] += 1;
        }
    }

    blotto    
}

// TODO: conv blotto