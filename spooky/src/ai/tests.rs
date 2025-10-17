use std::marker::PhantomData;

use bumpalo::Bump;

use crate::{
    ai::{
        captain::board_node::{BoardChildGen, BoardNodeArgs, BoardNodeState},
        mcts::ChildGen,
    },
    core::{
        board::{actions::BoardTurn, Board},
        game::GameConfig,
        map::Map,
        side::Side,
        tech::{Tech, TechAssignment, TechState, Techline},
        units::Unit,
    },
    heuristics::{naive::NaiveHeuristic, Heuristic},
    utils::make_rng,
};

#[test]
fn test_propose_move_integration() {
    let map = Map::default();
    let mut board = Board::new(&map);
    let side = Side::Yellow;

    // Add a necromancer to the board so there are spawnable units
    board.add_piece(crate::core::board::Piece::new(
        crate::core::units::Unit::BasicNecromancer,
        side,
        crate::core::loc::Loc::new(1, 1),
    ));

    // Add an enemy piece close enough for combat
    board.add_piece(crate::core::board::Piece::new(
        crate::core::units::Unit::Zombie,
        !side,
        crate::core::loc::Loc::new(1, 2), // Adjacent to necromancer
    ));

    // Set board to Normal state which includes both Attack and Spawn phases
    board.state = crate::core::board::definitions::BoardState::Normal;

    // Debug: Check board state
    println!("Board phases: {:?}", board.state.phases());
    println!("Board pieces: {:?}", board.pieces);

    let config = GameConfig::default();
    let node_state = BoardNodeState::new(board, side, &NaiveHeuristic::new(&config));
    let mut rng = make_rng();
    let mut tech_state = TechState::new();
    let techline = Techline::default();

    // Unlock more techs to ensure we have spawnable units
    let assignment = TechAssignment::new(10, vec![0, 1, 2]); // Unlock first 3 techs
    tech_state
        .assign_techs(assignment, side, &techline)
        .unwrap();

    let args = BoardNodeArgs {
        money: 100,
        tech_state,
        config: &config,
        arena: &Bump::new(),
        heuristic: &NaiveHeuristic::new(&config),
        _c: PhantomData,
    };

    let mut child_gen = BoardChildGen::new(&node_state, &mut rng, args.clone());

    let (turn, new_state) = child_gen
        .propose_turn(&node_state, &mut rng, args)
        .expect("Failed to propose turn");

    let board_actions = match turn {
        BoardTurn::Resign => panic!("Resignation not expected"),
        BoardTurn::Actions(board_actions) => board_actions,
    };

    // Debug output
    println!("Attack actions: {:?}", board_actions.attack_actions);
    println!("Spawn actions: {:?}", board_actions.spawn_actions);

    // Basic sanity checks
    assert!(!board_actions.attack_actions.is_empty() || !board_actions.spawn_actions.is_empty());
    assert_eq!(new_state.side_to_move, !side);
}
