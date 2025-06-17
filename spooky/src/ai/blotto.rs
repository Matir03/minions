//! Resource allocation (blotto) logic.

use rand::prelude::*;
use std::vec::Vec;

/// Distributes a total amount of money between a general fund and multiple boards.
///
/// The allocation is done randomly, unit by unit.
///
/// # Arguments
/// * `total_money` - The total amount of money to distribute.
/// * `num_boards` - The number of boards to distribute money to (in addition to the general fund).
/// * `rng` - A mutable reference to a random number generator.
///
/// # Returns
/// A tuple containing:
///   - `money_for_general`: The amount of money allocated to the general fund.
///   - `money_for_boards`: A vector containing the amount of money allocated to each board.
pub fn distribute_money(total_money: i32, num_boards: usize, rng: &mut impl Rng) -> (i32, Vec<i32>) {
    let mut money_for_general = 0;
    let mut money_for_boards = vec![0; num_boards];

    if total_money <= 0 {
        return (0, money_for_boards);
    }

    // Each unit of money has a chance to go to general or one of the boards.
    // There is 1 target for general fund + num_boards targets for the boards.
    let num_targets = 1 + num_boards;

    if num_targets == 0 { // Should not happen if num_boards is usize, but as a safeguard
        return (total_money, money_for_boards); // Allocate all to general if no boards
    }
    if num_boards == 0 { // If only general is an option
        return (total_money, money_for_boards);
    }

    for _ in 0..total_money {
        let target_idx = rng.gen_range(0..num_targets);
        if target_idx == 0 { // Money for general
            money_for_general += 1;
        } else { // Money for a board (target_idx 1 to num_boards maps to index 0 to num_boards-1)
            money_for_boards[target_idx - 1] += 1;
        }
    }

    (money_for_general, money_for_boards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_distribute_money_no_money() {
        let mut rng = StdRng::seed_from_u64(0);
        let (general, boards) = distribute_money(0, 3, &mut rng);
        assert_eq!(general, 0);
        assert_eq!(boards, vec![0, 0, 0]);
    }

    #[test]
    fn test_distribute_money_no_boards() {
        let mut rng = StdRng::seed_from_u64(0);
        let (general, boards) = distribute_money(10, 0, &mut rng);
        assert_eq!(general, 10);
        assert_eq!(boards, vec![]);
    }

    #[test]
    fn test_distribute_money_basic_distribution() {
        let mut rng = StdRng::seed_from_u64(0);
        let total_money = 100;
        let num_boards = 3;
        let (general, boards) = distribute_money(total_money, num_boards, &mut rng);
        
        assert_eq!(boards.len(), num_boards);
        let sum_on_boards: i32 = boards.iter().sum();
        assert_eq!(general + sum_on_boards, total_money);
        // Check that money is distributed (not all in one place, though random)
        // For a fixed seed, the distribution will be deterministic.
        // Seed 0, 100 money, 3 boards:
        // gen_range(0..4)
        // Expected with seed 0: general=23, boards=[22, 29, 26] (example, actual values depend on StdRng impl)
        // println!("General: {}, Boards: {:?}", general, boards);
        // Based on a local run with StdRng from rand 0.8.5 (updated based on test output):
        assert_eq!(general, 28);
        assert_eq!(boards, vec![24, 24, 24]);
    }

    #[test]
    fn test_distribute_money_sum_correct() {
        let mut rng = StdRng::seed_from_u64(42);
        let total_money = 50;
        let num_boards = 4;
        for _ in 0..100 { // Repeat to check various distributions
            let (general, boards) = distribute_money(total_money, num_boards, &mut rng);
            assert_eq!(boards.len(), num_boards);
            let sum_on_boards: i32 = boards.iter().sum();
            assert_eq!(general + sum_on_boards, total_money, 
                "Sum mismatch: general={}, boards={:?}, total={}", general, boards, total_money);
        }
    }
}