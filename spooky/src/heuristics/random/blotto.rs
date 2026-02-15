use crate::core::Blotto;
use crate::heuristics::BlottoGen;
use rand::distr::{weighted::WeightedIndex, Distribution, Uniform};
use rand::prelude::*;

pub struct RandomBlotto {
    pub total_money: i32,
    pub num_boards: usize,
}

impl<'a> BlottoGen<'a> for RandomBlotto {
    fn blotto(&self, money_for_spells: i32) -> Blotto {
        let remaining_money = self.total_money - money_for_spells;
        let mut money_for_boards = vec![0; self.num_boards];

        if remaining_money <= 0 {
            return Blotto { money_for_boards };
        }
        let mut rng = rand::rng();
        for _ in 0..remaining_money {
            let board_idx = rng.random_range(0..self.num_boards);
            money_for_boards[board_idx] += 1;
        }

        Blotto { money_for_boards }
    }
}
