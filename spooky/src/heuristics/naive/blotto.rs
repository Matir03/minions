//! Resource allocation (blotto) logic for naive heuristics.

use crate::core::{Blotto, GameState};
use crate::heuristics::BlottoGen;

pub struct NaiveBlotto {
    pub total_money: i32,
    pub num_boards: usize,
}

impl<'a> BlottoGen<'a> for NaiveBlotto {
    fn blotto(&self, money_for_spells: i32) -> Blotto {
        blotto(self.total_money, money_for_spells, self.num_boards)
    }
}

/// Distributes a total amount of money between a general fund and multiple boards.
///
/// The allocation is deterministic. The general fund receives a specified amount for spells,
/// and the rest is distributed as evenly as possible among the boards.
///
/// # Arguments
/// * `total_money` - The total amount of money to distribute.
/// * `money_for_spells` - The amount of money to be allocated to the general fund for spells.
/// * `num_boards` - The number of boards to distribute the remaining money to.
///
/// # Returns
/// A `Blotto` struct containing:
///   - `money_for_general`: The amount of money allocated to the general fund.
///   - `money_for_boards`: A vector containing the amount of money allocated to each board.
fn blotto(total_money: i32, money_for_spells: i32, num_boards: usize) -> Blotto {
    let mut money_for_boards = vec![0; num_boards];

    let remaining_money = total_money - money_for_spells;
    if remaining_money <= 0 {
        return Blotto { money_for_boards };
    }

    let base_amount = remaining_money / num_boards as i32;
    let mut remainder = remaining_money % num_boards as i32;

    for board_money in money_for_boards.iter_mut() {
        *board_money = base_amount;
        if remainder > 0 {
            *board_money += 1;
            remainder -= 1;
        }
    }

    Blotto { money_for_boards }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distribute_money_no_money() {
        let blotto = blotto(0, 0, 3);
        assert_eq!(blotto.money_for_boards, vec![0, 0, 0]);
    }

    #[test]
    fn test_distribute_money_only_spells() {
        let blotto = blotto(20, 20, 3);
        assert_eq!(blotto.money_for_boards, vec![0, 0, 0]);
    }

    #[test]
    fn test_distribute_money_even_split() {
        let blotto = blotto(100, 10, 3);
        assert_eq!(blotto.money_for_boards, vec![30, 30, 30]);
    }

    #[test]
    fn test_distribute_money_with_remainder() {
        let blotto = blotto(100, 10, 4);
        // 90 remaining for 4 boards. 90 / 4 = 22 with remainder 2.
        assert_eq!(blotto.money_for_boards, vec![23, 23, 22, 22]);
    }

    #[test]
    fn test_distribute_money_insufficient_funds() {
        // Not enough money for the requested spells.
        let blotto = blotto(10, 20, 3);
        assert_eq!(blotto.money_for_boards, vec![0, 0, 0]);
    }
}
