//! Resource allocation (blotto) logic.

use std::vec::Vec;

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
/// A tuple containing:
///   - `money_for_general`: The amount of money allocated to the general fund.
///   - `money_for_boards`: A vector containing the amount of money allocated to each board.
pub fn distribute_money(
    total_money: i32,
    money_for_spells: i32,
    num_boards: usize,
) -> (i32, Vec<i32>) {
    let money_for_general = money_for_spells;
    let mut money_for_boards = vec![0; num_boards];

    let remaining_money = total_money - money_for_general;
    if remaining_money <= 0 {
        return (money_for_general, money_for_boards);
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

    (money_for_general, money_for_boards)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distribute_money_no_money() {
        let (general, boards) = distribute_money(0, 0, 3);
        assert_eq!(general, 0);
        assert_eq!(boards, vec![0, 0, 0]);
    }

    #[test]
    fn test_distribute_money_only_spells() {
        let (general, boards) = distribute_money(20, 20, 3);
        assert_eq!(general, 20);
        assert_eq!(boards, vec![0, 0, 0]);
    }

    #[test]
    fn test_distribute_money_even_split() {
        let (general, boards) = distribute_money(100, 10, 3);
        assert_eq!(general, 10);
        assert_eq!(boards, vec![30, 30, 30]);
    }

    #[test]
    fn test_distribute_money_with_remainder() {
        let (general, boards) = distribute_money(100, 10, 4);
        // 90 remaining for 4 boards. 90 / 4 = 22 with remainder 2.
        assert_eq!(general, 10);
        assert_eq!(boards, vec![23, 23, 22, 22]);
    }

    #[test]
    fn test_distribute_money_no_boards() {
        let (general, boards) = distribute_money(50, 10, 0);
        assert_eq!(general, 50);
        assert!(boards.is_empty());
    }

    #[test]
    fn test_distribute_money_insufficient_funds() {
        // Not enough money for the requested spells.
        let (general, boards) = distribute_money(10, 20, 3);
        assert_eq!(general, 20);
        assert_eq!(boards, vec![0, 0, 0]);
    }
}
