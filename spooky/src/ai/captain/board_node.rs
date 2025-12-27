use std::io::Write;
use std::marker::PhantomData;
use std::ops::AddAssign;
/// MCTS Node representing the state of a single board and its turn processing (attack + spawn).
use std::{cell::RefCell, fmt};

use derive_where::derive_where;
use rand::prelude::*;

use crate::ai::captain::positioning::MoveCandidate;
use crate::core::board::actions::BoardActions;
use crate::core::board::definitions::Phase;
use crate::core::{Loc, ToIndex, Unit};
use crate::heuristics::BoardHeuristic;

#[derive(Debug, Clone)]
pub enum RemovalAssumption {
    Kill(Loc),
    Unsummon(Loc),
    Keep(Loc),
}

impl RemovalAssumption {
    pub fn loc(&self) -> Loc {
        match self {
            RemovalAssumption::Kill(loc) => *loc,
            RemovalAssumption::Unsummon(loc) => *loc,
            RemovalAssumption::Keep(loc) => *loc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoardAttackPhasePreTurn {
    pub moves: Vec<MoveCandidate>,
    pub removals: Vec<RemovalAssumption>,
}

pub type BoardSetupPhasePreTurn = Option<SetupAction>;
pub type BoardSpawnPhasePreTurn = Vec<SpawnAction>;

use crate::{
    ai::{
        eval::Eval,
        mcts::{ChildGen, MCTSNode, NodeStats},
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
    },
    positioning::SatPositioningSystem,
    // removing generate_heuristic_spawn_actions
};

use bumpalo::Bump;
use z3::{Config, Context, SatResult};

/// Represents the state of a single board and its turn processing (attack + spawn).
#[derive_where(Clone, PartialEq, Eq)]
pub struct BoardNodeState<'a, C, H: BoardHeuristic<'a, C>> {
    #[derive_where(skip)]
    pub heuristic_state: H::BoardEnc,
    pub board: Board<'a>,
    pub side_to_move: Side,
    pub delta_money: SideArray<i32>,
    pub delta_points: SideArray<i32>,
}

impl<'a, C, H: BoardHeuristic<'a, C>> BoardNodeState<'a, C, H> {
    pub fn new(board: Board<'a>, side_to_move: Side, heuristic: &H) -> Self {
        Self {
            heuristic_state: heuristic.compute_enc(&board),
            board,
            side_to_move,
            delta_money: SideArray::new(0, 0),
            delta_points: SideArray::new(0, 0),
        }
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
pub struct BoardChildGen<'a> {
    pub manager: ConstraintManager<'a>,
    pub positioning_system: SatPositioningSystem,
    pub combat_generator: CombatGenerationSystem,
    pub resign_generated: bool,
    pub resign_weight: f64,
}

pub fn process_turn_end<'a, C, H: BoardHeuristic<'a, C>>(
    mut board: Board<'a>,
    side_to_move: Side,
    money: i32,
    money_after_spawn: i32,
    rebate: i32,
    heuristic: &H,
    enc: &H::BoardEnc,
    turn: &BoardTurn,
) -> BoardNodeState<'a, C, H> {
    let (income, winner) = board
        .end_turn(side_to_move)
        .expect("[BoardNodeState] Failed to end turn");

    let mut delta_points = SideArray::new(0, 0);
    if let Some(winner) = winner {
        delta_points[winner] = 1;
    }

    let mut delta_money = SideArray::new(0, 0);
    delta_money[side_to_move] = money_after_spawn - money + income;
    delta_money[!side_to_move] = rebate;

    let new_state = BoardNodeState {
        board,
        side_to_move: !side_to_move,
        delta_money,
        delta_points,
        heuristic_state: heuristic.update_enc(enc, turn),
    };

    new_state
}

#[derive_where(Clone)]
pub struct BoardNodeArgs<'a, C: Clone, H: BoardHeuristic<'a, C>> {
    pub money: i32,
    pub tech_state: TechState,
    pub config: &'a GameConfig,
    pub arena: &'a Bump,
    pub heuristic: &'a H,
    pub shared: C,
}

impl<'a, C: Clone, H: 'a + BoardHeuristic<'a, C>> ChildGen<BoardNodeState<'a, C, H>, BoardTurn>
    for BoardChildGen<'a>
{
    type Args = BoardNodeArgs<'a, C, H>;

    fn new(state: &BoardNodeState<'a, C, H>, rng: &mut impl Rng, args: Self::Args) -> Self {
        let config = args.config;
        let arena = args.arena;
        let heuristic = args.heuristic;
        let shared = args.shared;

        let z3_cfg = Config::new();
        let ctx = arena.alloc(Context::new(&z3_cfg));
        let mut new_board = state.board.clone();

        // --- Initialize systems ---
        let (mut manager, positioning_system, mut combat_generator) = run_timed(
            "Initializing attack phase systems",
            || {
                let graph = new_board.combat_graph(state.side_to_move);
                let manager = ConstraintManager::new(ctx, graph, &new_board);
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

        BoardChildGen {
            manager,
            positioning_system,
            combat_generator,
            resign_generated: false,
            resign_weight: 0.0,
        }
    }

    fn propose_turn(
        &mut self,
        state: &BoardNodeState<'a, C, H>,
        rng: &mut impl Rng,
        args: Self::Args,
    ) -> Option<(BoardTurn, BoardNodeState<'a, C, H>)> {
        let BoardNodeArgs {
            money,
            tech_state,
            config,
            arena,
            heuristic,
            shared,
        } = args;

        let BoardChildGen {
            ref mut manager,
            ref mut positioning_system,
            ref mut combat_generator,
            resign_generated,
            resign_weight,
        } = *self;

        let mut new_board = state.board.clone();

        if !resign_generated {
            if rng.gen::<f64>() < resign_weight {
                self.resign_generated = true;

                new_board.take_turn(state.side_to_move, BoardTurn::Resign, money, &tech_state);
                let new_node = process_turn_end(
                    new_board,
                    state.side_to_move,
                    money,
                    money,
                    0,
                    heuristic,
                    &state.heuristic_state,
                    &BoardTurn::Resign,
                );

                return Some((BoardTurn::Resign, new_node));
            }
        }

        let setup_action = heuristic.compute_board_setup_phase_pre_turn(
            rng,
            &shared,
            &state.heuristic_state,
            &new_board,
            state.side_to_move,
        );

        if let Some(ref pre_turn) = setup_action {
            new_board
                .do_setup_action(state.side_to_move, pre_turn)
                .unwrap();
        }

        let BoardAttackPhasePreTurn { moves, removals } = run_timed(
            "compute attack phase pre turn",
            || {
                heuristic.compute_board_attack_phase_pre_turn(
                    rng,
                    &shared,
                    &state.heuristic_state,
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
                        .generate_combat(manager, &new_board, state.side_to_move, &removals)
                        .unwrap();
                },
                |_| "done".to_string(),
            );

            // --- Combat Reconciliation ---
            let result = run_timed(
                "Combat reconciliation",
                || {
                    positioning_system
                        .combat_reconciliation(manager, &new_board, state.side_to_move, &moves)
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
                    .combat_positioning(manager, &new_board, state.side_to_move, &moves)
                    .expect("Attack positioning error");
            },
            |_| "done".to_string(),
        );

        // --- Extract all moves from the combined model ---
        let (rebate, combat_actions) = run_timed(
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
                        new_board
                            .do_attacks(state.side_to_move, &combat_actions)
                            .unwrap(),
                        combat_actions,
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
                let actions =
                    positioning_system.generate_non_attack_movements(&manager, &new_board, moves);
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
                    let spawn_actions = heuristic.compute_board_spawn_phase_pre_turn(
                        rng,
                        &shared,
                        &state.heuristic_state,
                        &new_board,
                        state.side_to_move,
                        money,
                        &tech_state,
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

        let attack_actions = combat_actions
            .into_iter()
            .chain(positioning_actions.into_iter())
            .collect::<Vec<_>>();

        let turn_taken = BoardTurn::Actions(BoardActions {
            setup_action,
            attack_actions,
            spawn_actions,
        });

        let new_node = process_turn_end(
            new_board,
            state.side_to_move,
            money,
            money_after_spawn,
            rebate,
            heuristic,
            &state.heuristic_state,
            &turn_taken,
        );

        Some((turn_taken, new_node))
    }
}

pub type BoardNode<'a, C, H> = MCTSNode<'a, BoardNodeState<'a, C, H>, BoardTurn, BoardChildGen<'a>>;
pub type BoardNodeRef<'a, C, H> = &'a RefCell<BoardNode<'a, C, H>>;

#[cfg(test)]
mod tests {
    use crate::core::board::definitions::BoardState;
    use std::marker::PhantomData;
    use std::rc::Rc;

    use crate::{
        ai::{
            captain::board_node::{BoardChildGen, BoardNodeArgs},
            mcts::ChildGen,
        },
        core::{
            board::{
                actions::{AttackAction, BoardTurn},
                Board,
            },
            game::GameConfig,
            map::{Map, MapSpec},
            side::Side,
            tech::{Tech, TechAssignment, TechState, Techline},
        },
        heuristics::{naive::NaiveHeuristic, Heuristic},
    };
    use bumpalo::Bump;
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
        let mut board = Board::from_fen(Board::START_FEN, &map).unwrap();
        board.state = BoardState::Normal;
        let mut rng = thread_rng();
        let tech_state = new_all_unlocked_tech_state();
        let config = GameConfig::default();
        let heuristic = NaiveHeuristic::new(&config);
        let node_state = BoardNodeState::new(board.clone(), Side::Yellow, &heuristic);
        let game_state = crate::core::game::GameState {
            config: &config,
            game_id: "test".to_string(),
            tech_state: tech_state.clone(),
            side_to_move: Side::Yellow,
            boards: vec![board.clone()],
            board_points: crate::core::side::SideArray::new(0, 0),
            money: crate::core::side::SideArray::new(12, 12),
            ply: 0,
            winner: None,
        };
        let shared = Rc::new(game_state);
        let args = BoardNodeArgs {
            money: 12,
            tech_state: tech_state.clone(),
            config: &config,
            arena: &Bump::new(),
            heuristic: &heuristic,
            shared: shared.clone(),
        };
        let mut child_gen = BoardChildGen::new(&node_state, &mut rng, args.clone());

        let (turn, _new_state) = child_gen
            .propose_turn(&node_state, &mut rng, args)
            .expect("Failed to propose turn");

        let board_actions = match turn {
            BoardTurn::Resign => panic!("Resignation not expected"),
            BoardTurn::Actions(board_actions) => board_actions,
        };

        assert!(!board_actions.spawn_actions.is_empty());
    }
}
