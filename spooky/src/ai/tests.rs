use crate::{
    ai::captain::BoardNodeState,
    core::{
        board::Board,
        game::GameConfig,
        side::Side,
        tech::{Tech, TechAssignment, TechState, Techline},
        units::Unit,
    },
};
use rand::thread_rng;
use crate::ai::mcts::NodeState;

#[test]
fn test_propose_move_integration() {
    let board = Board::default();
    let side = Side::S0;
    let node_state = BoardNodeState::new(board, side);
    let mut rng = thread_rng();
    let mut tech_state = TechState::new();
    let techline = Techline::default();
    let assignment = TechAssignment::new(1, vec![0]);
    tech_state
        .assign_techs(assignment, side, &techline)
        .unwrap();
    let config = GameConfig::default();
    let (tech_state, config) = (tech_state, config);
    let args = (12, tech_state, config, 0, 0);

    let (turn, new_state) = node_state.propose_move(&mut rng, &args);

    // Basic sanity checks
    assert!(!turn.attack_actions.is_empty() || !turn.spawn_actions.is_empty());
    assert_eq!(new_state.side_to_move, side);
}
