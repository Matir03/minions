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
        mcts::{MCTSNode, NodeState, NodeStats},
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
        generation::CombatGenerationSystem,
        graph::CombatGraph,
        manager::{generate_move_from_sat_model, CombatManager},
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

impl<'a> NodeState<BoardTurn> for BoardNodeState<'a> {
    // Args: (money, tech_state, config, current_board_idx, turn_num)
    type Args = (i32, TechState, &'a GameConfig, usize, i32);

    fn propose_move(&self, rng: &mut impl Rng, args: &Self::Args) -> (BoardTurn, Self) {
        let (money, ref tech_state, _config, _current_board_idx, turn_num) = *args;

        let z3_cfg = Config::new();
        let ctx = Context::new(&z3_cfg);
        let mut new_board = self.board.clone();

        // --- Setup Phase ---
        let setup_phase_start = std::time::Instant::now();
        print!("Setup phase: ");
        let setup_action = if self.board.state.phases().contains(&Phase::Setup) {
            let action = setup_phase(self.side_to_move, &self.board);
            new_board
                .do_setup_action(self.side_to_move, &action)
                .unwrap();
            print!("done");
            Some(action)
        } else {
            print!("skipped");
            None
        };
        println!(" ({:.2?})", setup_phase_start.elapsed());

        // --- Combat Solver with Constraints (reused) ---
        let combat_start = std::time::Instant::now();
        print!("Combat phase: creating SAT solver ");
        std::io::stdout().flush().unwrap();

        let graph = new_board.combat_graph(self.side_to_move);
        let mut manager = CombatManager::new(&ctx, graph, &new_board);
        println!(" ({:.2?})", combat_start.elapsed());

        // --- Death Prophet (reused) ---
        let prophet_start = std::time::Instant::now();
        print!("Death Prophet: initializing ");
        std::io::stdout().flush().unwrap();

        let positioning_system = SatPositioningSystem {};
        let mut combat_generation = CombatGenerationSystem::new();
        println!(" ({:.2?})", prophet_start.elapsed());

        // First sat check for timining
        let sat_start = std::time::Instant::now();
        print!("First sat check: ");
        std::io::stdout().flush().unwrap();
        let sat_result = manager.solver.check();
        println!("done ({:.2?})", sat_start.elapsed());

        // --- Combat Generation (add death prophet assumptions) ---
        let generation_start = std::time::Instant::now();
        print!("Combat generation: ");
        std::io::stdout().flush().unwrap();

        combat_generation
            .generate_combat(&mut manager, &new_board, self.side_to_move)
            .unwrap();
        print!("done");
        println!(" ({:.2?})", generation_start.elapsed());

        // --- Generate move candidates ---
        let candidate_gen = std::time::Instant::now();
        print!("Generating move candidates: ");
        std::io::stdout().flush().unwrap();
        let move_candidates =
            positioning_system.generate_move_candidates(rng, &new_board, self.side_to_move);
        println!("done ({:.2?})", candidate_gen.elapsed());

        // --- Piece Positioning (add positioning constraints) ---
        let positioning_start = std::time::Instant::now();
        print!("Piece positioning: ");
        std::io::stdout().flush().unwrap();

        positioning_system.position_pieces_with_sat(
            &mut manager,
            &new_board,
            self.side_to_move,
            &move_candidates,
        );

        print!("done");
        println!(" ({:.2?})", positioning_start.elapsed());

        // --- Extract all moves from the combined model ---
        let combat_actions = match manager.solver.check() {
            z3::SatResult::Sat => {
                let model = manager
                    .solver
                    .get_model()
                    .expect("Failed to get model from SAT solver");

                generate_move_from_sat_model(&model, &manager.graph, &manager.variables, &new_board)
            }
            _ => panic!("Failed to generate combat actions"),
        };

        // --- Apply combat actions ---
        let combat_start = std::time::Instant::now();
        print!("Applying combat actions: ");
        std::io::stdout().flush().unwrap();
        let rebate = new_board
            .do_attacks(self.side_to_move, &combat_actions)
            .unwrap();
        println!("done ({:.2?})", combat_start.elapsed());

        // --- Generate non-attack movements ---
        let positioning_start = std::time::Instant::now();
        print!("Generating non-attack movements: ");
        std::io::stdout().flush().unwrap();
        let positioning_actions = positioning_system.generate_non_attack_movements(
            &manager,
            &mut new_board,
            self.side_to_move,
            move_candidates,
        );
        println!("{:?}", positioning_actions);
        new_board
            .do_attacks(self.side_to_move, &positioning_actions)
            .unwrap();
        println!("done ({:.2?})", positioning_start.elapsed());

        let attack_actions = combat_actions
            .into_iter()
            .chain(positioning_actions.into_iter())
            .collect::<Vec<_>>();

        // --- Spawning ---
        let spawn_start = std::time::Instant::now();
        print!("Spawn phase: ");
        std::io::stdout().flush().unwrap();
        let spawn_actions = if new_board.state.phases().contains(&Phase::Spawn) {
            generate_heuristic_spawn_actions(&new_board, self.side_to_move, tech_state, money, rng)
        } else {
            Vec::new()
        };

        let money_after_spawn = new_board
            .do_spawns(self.side_to_move, money, &spawn_actions)
            .expect(&format!(
                "[BoardNodeState] Failed to perform spawn phase actions: {:#?}",
                spawn_actions,
            ));
        println!("done ({:.2?})", spawn_start.elapsed());

        let (income, winner) = new_board
            .end_turn(self.side_to_move)
            .expect("[BoardNodeState] Failed to end turn");

        let mut delta_money = SideArray::new(0, 0);
        delta_money[self.side_to_move] = money_after_spawn - money + income;
        delta_money[!self.side_to_move] = rebate;

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

pub type BoardNode<'a> = MCTSNode<'a, BoardNodeState<'a>, BoardTurn>;
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
