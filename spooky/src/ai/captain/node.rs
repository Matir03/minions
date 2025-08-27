use std::io::Write;
use std::ops::AddAssign;
/// MCTS Node representing the state of a single board and its turn processing (attack + spawn).
use std::{cell::RefCell, fmt};

use rand::prelude::*;

use crate::core::board::definitions::Phase;
use crate::core::Unit;
use crate::{
    ai::{
        eval::Eval,
        mcts::{ChildGen, MCTSNode, NodeState, NodeStats},
    },
    core::{
        board::{
            actions::{AttackAction, BoardTurn, SetupAction, SpawnAction},
            Board,
        },
        game::{GameConfig, GameState},
        map::Map,
        side::{Side, SideArray},
        tech::TechState,
    },
};

use super::{
    combat::{
        constraints::{generate_move_from_sat_model, ConstraintManager},
        generation::CombatGenerationSystem,
        graph::CombatGraph,
        prophet::DeathProphet,
    },
    positioning::SatPositioningSystem,
    spawn::generate_heuristic_spawn_actions,
};

use z3::{Config, Context, SatResult};

/// Represents the state of a single board and its turn processing (attack + spawn).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardNodeState<'a> {
    pub board: Board<'a>,
    pub side_to_move: Side,
    pub delta_money: SideArray<i32>,
    pub delta_points: SideArray<i32>,
}

impl<'a> BoardNodeState<'a> {
    pub fn new(board: Board<'a>, side_to_move: Side) -> Self {
        Self {
            board,
            side_to_move,
            delta_money: SideArray::new(0, 0),
            delta_points: SideArray::new(0, 0),
        }
    }
}

pub fn setup_phase(side: Side, board: &Board<'_>) -> SetupAction {
    let saved_unit = board.reinforcements[side]
        .iter()
        .max_by_key(|unit| unit.stats().cost)
        .cloned();

    SetupAction {
        necromancer_choice: Unit::ArcaneNecromancer,
        saved_unit,
    }
}

fn run_timed<T, F, G>(desc: &str, thunk: F, result_msg: G) -> T
where
    F: FnOnce() -> T,
    G: FnOnce(&T) -> String,
{
    use std::io::{self, Write};

    // Helper to select the debug output target
    fn debug_target() -> Box<dyn Write> {
        #[cfg(feature = "profiling")]
        {
            Box::new(io::stdout())
        }
        #[cfg(all(debug_assertions, not(feature = "profiling")))]
        {
            Box::new(io::stderr())
        }
        #[cfg(all(not(debug_assertions), not(feature = "profiling")))]
        {
            Box::new(io::sink())
        }
    }

    let start = std::time::Instant::now();
    let mut dbg_out = debug_target();
    write!(dbg_out, "{}: ", desc).ok();
    dbg_out.flush().ok();
    let result = thunk();
    let elapsed = start.elapsed();
    let msg = result_msg(&result);
    writeln!(dbg_out, "{} ({:.2?})", msg, elapsed).ok();
    result
}

// --------------------------
// Child generator for Board
// --------------------------

#[derive(Debug)]
pub struct BoardChildGen;

impl BoardChildGen {
    pub fn new() -> Self {
        Self
    }
}

impl<'a> ChildGen<BoardNodeState<'a>, BoardTurn> for BoardChildGen {
    type Args = (i32, TechState, &'a GameConfig, usize, i32);

    fn new(_state: &BoardNodeState<'a>) -> Self {
        BoardChildGen::new()
    }

    fn propose_move(
        &mut self,
        state: &BoardNodeState<'a>,
        rng: &mut impl Rng,
        args: &Self::Args,
    ) -> (BoardTurn, BoardNodeState<'a>) {
        let (money, ref tech_state, _config, _current_board_idx, turn_num) = *args;

        let z3_cfg = Config::new();
        let ctx = Context::new(&z3_cfg);
        let mut new_board = state.board.clone();

        // --- Setup Phase ---
        let setup_action = run_timed(
            "Setup phase",
            || {
                if state.board.state.phases().contains(&Phase::Setup) {
                    let action = setup_phase(state.side_to_move, &state.board);
                    new_board
                        .do_setup_action(state.side_to_move, &action)
                        .unwrap();
                    Some(action)
                } else {
                    None
                }
            },
            |result| {
                if result.is_some() {
                    "done".to_string()
                } else {
                    "skipped".to_string()
                }
            },
        );

        // --- Initialize systems ---
        let (mut manager, positioning_system, mut combat_generator) = run_timed(
            "Initializing attack phase systems",
            || {
                let graph = new_board.combat_graph(state.side_to_move);
                let manager = ConstraintManager::new(&ctx, graph, &new_board);
                let positioning_system = SatPositioningSystem {};
                let combat_generator = CombatGenerationSystem::new();
                (manager, positioning_system, combat_generator)
            },
            |_| "done".to_string(),
        );

        // First sat check for timing and pre-solving
        let sat_result = run_timed(
            "First sat check",
            || manager.solver.check(),
            |result| match result {
                z3::SatResult::Sat => "done".to_string(),
                _ => panic!("SAT check failed"),
            },
        );

        // --- Generate move candidates ---
        let move_candidates = run_timed(
            "Generating move candidates",
            || {
                positioning_system.generate_move_candidates(
                    rng,
                    &manager.graph,
                    &new_board,
                    state.side_to_move,
                )
            },
            |_| "done".to_string(),
        );

        // try combat generation + attack reconciliation until we succeed
        loop {
            // --- Combat Generation (add death prophet assumptions) ---
            run_timed(
                "Combat generation",
                || {
                    manager.solver.push();
                    combat_generator
                        .generate_combat(&mut manager, &new_board, state.side_to_move)
                        .unwrap();
                },
                |_| "done".to_string(),
            );

            // --- Combat Reconciliation ---
            let result = run_timed(
                "Combat reconciliation",
                || {
                    positioning_system
                        .combat_reconciliation(
                            &mut manager,
                            &new_board,
                            state.side_to_move,
                            &move_candidates,
                        )
                        .expect("Combat reconciliation error")
                },
                |res| {
                    if res.is_none() {
                        "success".to_string()
                    } else {
                        "backtracking".to_string()
                    }
                },
            );

            if result.is_none() {
                break;
            }

            // no legal way to resolve the conflicts, backtrack
            manager.solver.pop(1);

            // and add the new constraints to the solver
            for constraint in result.unwrap() {
                manager.solver.assert(&constraint);
            }
        }

        // --- Combat Positioning ---
        run_timed(
            "Combat positioning",
            || {
                positioning_system
                    .combat_positioning(
                        &mut manager,
                        &new_board,
                        state.side_to_move,
                        &move_candidates,
                    )
                    .expect("Attack positioning error");
            },
            |_| "done".to_string(),
        );

        // --- Extract all moves from the combined model ---
        let (combat_actions, rebate) = run_timed(
            "Extracting and applying combat actions",
            || match manager.solver.check() {
                z3::SatResult::Sat => {
                    let model = manager
                        .solver
                        .get_model()
                        .expect("Failed to get model from SAT solver");
                    let combat_actions = generate_move_from_sat_model(
                        &model,
                        &manager.graph,
                        &manager.variables,
                        &new_board,
                    );

                    (
                        combat_actions.clone(),
                        new_board
                            .do_attacks(state.side_to_move, &combat_actions)
                            .unwrap(),
                    )
                }
                _ => panic!("Failed to generate combat actions"),
            },
            |_| "done".to_string(),
        );

        // --- Generate non-combat movements ---
        let positioning_actions = run_timed(
            "Generating and applying non-combat movements",
            || {
                let actions = positioning_system.generate_non_attack_movements(
                    &manager,
                    &new_board,
                    move_candidates,
                );
                new_board.do_attacks(state.side_to_move, &actions).unwrap();
                actions
            },
            |_| "done".to_string(),
        );

        // --- Spawning ---
        let (spawn_actions, money_after_spawn) = run_timed(
            "Spawn phase",
            || {
                if new_board.state.phases().contains(&Phase::Spawn) && new_board.winner.is_none() {
                    let spawn_actions = generate_heuristic_spawn_actions(
                        &new_board,
                        state.side_to_move,
                        tech_state,
                        money,
                        rng,
                    );
                    let money_after_spawn = new_board
                        .do_spawns(state.side_to_move, money, &spawn_actions, &tech_state)
                        .unwrap();
                    (spawn_actions, money_after_spawn)
                } else {
                    (Vec::new(), money)
                }
            },
            |_| "done".to_string(),
        );

        let (income, winner) = new_board
            .end_turn(state.side_to_move)
            .expect("[BoardNodeState] Failed to end turn");

        let mut delta_money = SideArray::new(0, 0);
        delta_money[state.side_to_move] = money_after_spawn - money + income;
        delta_money[!state.side_to_move] = rebate;

        let attack_actions = combat_actions
            .into_iter()
            .chain(positioning_actions.into_iter())
            .collect::<Vec<_>>();

        let turn_taken = BoardTurn {
            setup_action,
            attack_actions,
            spawn_actions,
        };

        let mut delta_points = SideArray::new(0, 0);
        if let Some(winner) = winner {
            delta_points[winner] = 1;
        }

        let new_state = BoardNodeState {
            board: new_board,
            side_to_move: !state.side_to_move,
            delta_money,
            delta_points,
        };

        (turn_taken, new_state)
    }
}

impl<'a> NodeState<BoardTurn> for BoardNodeState<'a> {
    // Args: (money, tech_state, config, current_board_idx, turn_num)
    type Args = (i32, TechState, &'a GameConfig, usize, i32);

    fn propose_move(&self, rng: &mut impl Rng, args: &Self::Args) -> (BoardTurn, Self) {
        let (money, ref tech_state, _config, _current_board_idx, turn_num) = *args;

        let z3_cfg = Config::new();
        let ctx = Context::new(&z3_cfg);
        let mut new_board = self.board.clone();

        // --- Setup Phase ---
        let setup_action = run_timed(
            "Setup phase",
            || {
                if self.board.state.phases().contains(&Phase::Setup) {
                    let action = setup_phase(self.side_to_move, &self.board);
                    new_board
                        .do_setup_action(self.side_to_move, &action)
                        .unwrap();
                    Some(action)
                } else {
                    None
                }
            },
            |result| {
                if result.is_some() {
                    "done".to_string()
                } else {
                    "skipped".to_string()
                }
            },
        );

        // --- Initialize systems ---
        let (mut manager, positioning_system, mut combat_generator) = run_timed(
            "Initializing attack phase systems",
            || {
                let graph = new_board.combat_graph(self.side_to_move);
                let manager = ConstraintManager::new(&ctx, graph, &new_board);
                let positioning_system = SatPositioningSystem {};
                let combat_generator = CombatGenerationSystem::new();
                (manager, positioning_system, combat_generator)
            },
            |_| "done".to_string(),
        );

        // First sat check for timing and pre-solving
        let sat_result = run_timed(
            "First sat check",
            || manager.solver.check(),
            |result| match result {
                z3::SatResult::Sat => "done".to_string(),
                _ => panic!("SAT check failed"),
            },
        );

        // --- Generate move candidates ---
        let move_candidates = run_timed(
            "Generating move candidates",
            || {
                positioning_system.generate_move_candidates(
                    rng,
                    &manager.graph,
                    &new_board,
                    self.side_to_move,
                )
            },
            |_| "done".to_string(),
        );

        // try combat generation + attack reconciliation until we succeed
        loop {
            // --- Combat Generation (add death prophet assumptions) ---
            run_timed(
                "Combat generation",
                || {
                    manager.solver.push();
                    combat_generator
                        .generate_combat(&mut manager, &new_board, self.side_to_move)
                        .unwrap();
                },
                |_| "done".to_string(),
            );

            // --- Combat Reconciliation ---
            let result = run_timed(
                "Combat reconciliation",
                || {
                    positioning_system
                        .combat_reconciliation(
                            &mut manager,
                            &new_board,
                            self.side_to_move,
                            &move_candidates,
                        )
                        .expect("Combat reconciliation error")
                },
                |res| {
                    if res.is_none() {
                        "success".to_string()
                    } else {
                        "backtracking".to_string()
                    }
                },
            );

            if result.is_none() {
                break;
            }

            // no legal way to resolve the conflicts, backtrack
            manager.solver.pop(1);

            // and add the new constraints to the solver
            for constraint in result.unwrap() {
                manager.solver.assert(&constraint);
            }
        }

        // --- Combat Positioning ---
        run_timed(
            "Combat positioning",
            || {
                positioning_system
                    .combat_positioning(
                        &mut manager,
                        &new_board,
                        self.side_to_move,
                        &move_candidates,
                    )
                    .expect("Attack positioning error");
            },
            |_| "done".to_string(),
        );

        // --- Extract all moves from the combined model ---
        let (combat_actions, rebate) = run_timed(
            "Extracting and applying combat actions",
            || match manager.solver.check() {
                z3::SatResult::Sat => {
                    let model = manager
                        .solver
                        .get_model()
                        .expect("Failed to get model from SAT solver");
                    let combat_actions = generate_move_from_sat_model(
                        &model,
                        &manager.graph,
                        &manager.variables,
                        &new_board,
                    );

                    (
                        combat_actions.clone(),
                        new_board
                            .do_attacks(self.side_to_move, &combat_actions)
                            .unwrap(),
                    )
                }
                _ => panic!("Failed to generate combat actions"),
            },
            |_| "done".to_string(),
        );

        // --- Generate non-combat movements ---
        let positioning_actions = run_timed(
            "Generating and applying non-combat movements",
            || {
                let actions = positioning_system.generate_non_attack_movements(
                    &manager,
                    &new_board,
                    move_candidates,
                );
                new_board.do_attacks(self.side_to_move, &actions).unwrap();
                actions
            },
            |_| "done".to_string(),
        );

        // --- Spawning ---
        let (spawn_actions, money_after_spawn) = run_timed(
            "Spawn phase",
            || {
                if new_board.state.phases().contains(&Phase::Spawn) && new_board.winner.is_none() {
                    let spawn_actions = generate_heuristic_spawn_actions(
                        &new_board,
                        self.side_to_move,
                        tech_state,
                        money,
                        rng,
                    );
                    let money_after_spawn = new_board
                        .do_spawns(self.side_to_move, money, &spawn_actions, &tech_state)
                        .unwrap();
                    (spawn_actions, money_after_spawn)
                } else {
                    (Vec::new(), money)
                }
            },
            |_| "done".to_string(),
        );

        let (income, winner) = new_board
            .end_turn(self.side_to_move)
            .expect("[BoardNodeState] Failed to end turn");

        let mut delta_money = SideArray::new(0, 0);
        delta_money[self.side_to_move] = money_after_spawn - money + income;
        delta_money[!self.side_to_move] = rebate;

        let attack_actions = combat_actions
            .into_iter()
            .chain(positioning_actions.into_iter())
            .collect::<Vec<_>>();

        let turn_taken = BoardTurn {
            setup_action,
            attack_actions,
            spawn_actions,
        };

        let mut delta_points = SideArray::new(0, 0);
        if let Some(winner) = winner {
            delta_points[winner] = 1;
        }

        let new_state = Self {
            board: new_board,
            side_to_move: !self.side_to_move,
            delta_money,
            delta_points,
        };

        (turn_taken, new_state)
    }
}

pub type BoardNode<'a> = MCTSNode<'a, BoardNodeState<'a>, BoardTurn, BoardChildGen>;
pub type BoardNodeRef<'a> = &'a RefCell<BoardNode<'a>>;

#[cfg(test)]
mod tests {
    use crate::ai::mcts::NodeState;
    use crate::core::{
        board::actions::AttackAction,
        board::Board,
        game::GameConfig,
        map::{Map, MapSpec},
        side::Side,
        tech::{Tech, TechAssignment, TechState, Techline},
    };
    use rand::thread_rng;

    use super::BoardNodeState;

    fn new_all_unlocked_tech_state() -> TechState {
        let mut tech_state = TechState::new();
        let techline = Techline::default();
        let mut acquire_indices = Vec::new();

        // Collect all 0-based indices for unit techs
        for i in 0..techline.len() {
            if let Tech::UnitTech(unit) = techline[i] {
                if unit.stats().cost > 0 {
                    acquire_indices.push(i);
                }
            }
        }

        // To acquire techs, we must first advance the unlock_index past them.
        let assignment_s0 = TechAssignment::new(techline.len(), acquire_indices.clone());
        let assignment_s1 = TechAssignment::new(techline.len(), acquire_indices);

        tech_state
            .assign_techs(assignment_s0, Side::Yellow, &techline)
            .unwrap();
        // Don't unwrap, as some techs may have been acquired by S0 already
        let _ = tech_state.assign_techs(assignment_s1, Side::Blue, &techline);

        tech_state
    }

    #[test]
    fn test_ai_proposes_non_trivial_opening_move() {
        let map = Map::BlackenedShores;
        let board = Board::from_fen(Board::START_FEN, &map).unwrap();
        let mut rng = thread_rng();
        let tech_state = new_all_unlocked_tech_state();
        let config = GameConfig::default();
        let node_state = BoardNodeState::new(board, Side::Yellow);

        let (turn, _new_state) =
            node_state.propose_move(&mut rng, &(12, tech_state, &config, 0, 0));

        assert!(!turn.spawn_actions.is_empty());
    }
}
