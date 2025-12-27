use rand::rngs::StdRng;
use rand::SeedableRng;
use spooky::core::{GameConfig, GameState, Side};
use spooky::heuristics::random::RandomHeuristic;
use spooky::heuristics::{GeneralHeuristic, Heuristic};

#[test]
fn test_random_heuristic_eval() {
    let config = GameConfig::default();
    let state = GameState::new_default(&config);
    let heuristic = RandomHeuristic::new(&config);

    let combined = heuristic.compute_combined(&state, &(), &[]);

    let eval1 = heuristic.compute_eval(&combined);
    let eval2 = heuristic.compute_eval(&combined);

    let score1 = eval1.score(Side::Yellow);
    let score2 = eval2.score(Side::Yellow);

    assert!(score1 >= 0.0 && score1 <= 1.0);
    assert!(score2 >= 0.0 && score2 <= 1.0);
}

#[test]
fn test_random_heuristic_pre_turn() {
    let config = GameConfig::default();
    let state = GameState::new_default(&config);
    let heuristic = RandomHeuristic::new(&config);

    let combined = heuristic.compute_combined(&state, &(), &[]);
    let mut rng = StdRng::seed_from_u64(42);

    let pre_turn = heuristic.compute_general_pre_turn(&mut rng, &combined, &());
    assert_eq!(pre_turn.weights.len(), config.techline.techs.len());
}
