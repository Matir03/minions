use crate::ai::captain::board_node::BoardAttackPhasePreTurn;
use crate::ai::captain::positioning::MoveCandidate;
use crate::core::{
    board::{
        actions::{BoardTurn, SetupAction, SpawnAction},
        definitions::Phase,
        BitboardOps,
    },
    tech::{Tech, TechState},
    Board, Side, ToIndex, Unit,
};
use crate::heuristics::{
    random::{CombinedEnc, RandomHeuristic},
    BoardHeuristic, BoardSetupPhasePreTurn, BoardSpawnPhasePreTurn, Heuristic, RemovalAssumption,
};
use rand::seq::SliceRandom;
use rand::Rng;

impl<'a> BoardHeuristic<'a, CombinedEnc<'a>> for RandomHeuristic<'a> {
    type BoardEnc = ();

    fn compute_enc(&self, _board: &Board<'a>) -> Self::BoardEnc {
        ()
    }

    fn update_enc(&self, _enc: &Self::BoardEnc, _turn: &BoardTurn) -> Self::BoardEnc {
        ()
    }

    fn compute_board_attack_phase_pre_turn(
        &self,
        rng: &mut impl Rng,
        _shared: &CombinedEnc<'a>,
        _enc: &Self::BoardEnc,
        board: &Board<'a>,
        side: Side,
    ) -> BoardAttackPhasePreTurn {
        let graph = board.combat_graph(side);
        let mut moves = Vec::new();

        for (from_loc, to_loc_map) in graph.move_hex_map.iter() {
            let unit_stats = board.get_piece(from_loc).unwrap().unit.stats();
            for to_loc in to_loc_map.keys() {
                moves.push(MoveCandidate::Move {
                    from_loc: *from_loc,
                    to_loc: *to_loc,
                    score: rng.gen(),
                });
            }
            if unit_stats.blink {
                moves.push(MoveCandidate::Blink {
                    loc: *from_loc,
                    score: rng.gen(),
                });
            }
        }

        let mut removals = Vec::new();
        for loc in &graph.defenders {
            removals.push(RemovalAssumption::Kill(*loc));
            removals.push(RemovalAssumption::Unsummon(*loc));
            removals.push(RemovalAssumption::Keep(*loc));
        }

        removals.shuffle(rng);

        BoardAttackPhasePreTurn { moves, removals }
    }

    fn compute_board_setup_phase_pre_turn(
        &self,
        rng: &mut impl Rng,
        _shared: &CombinedEnc<'a>,
        _enc: &Self::BoardEnc,
        board: &Board<'a>,
        side: Side,
    ) -> BoardSetupPhasePreTurn {
        if !board.state.phases().contains(&Phase::Setup) {
            return None;
        }

        let reinforcements: Vec<_> = board.reinforcements[side].iter().cloned().collect();
        let saved_unit = if reinforcements.is_empty() {
            None
        } else if rng.gen_bool(0.5) {
            Some(reinforcements.choose(rng).unwrap().clone())
        } else {
            None
        };

        Some(SetupAction {
            necromancer_choice: Unit::BasicNecromancer,
            saved_unit,
        })
    }

    fn compute_board_spawn_phase_pre_turn(
        &self,
        rng: &mut impl Rng,
        _shared: &CombinedEnc<'a>,
        _enc: &Self::BoardEnc,
        board: &Board<'a>,
        side: Side,
        mut money: i32,
        tech_state: &TechState,
    ) -> BoardSpawnPhasePreTurn {
        let mut actions = Vec::new();

        let available_units: Vec<_> = tech_state.acquired_techs[side]
            .iter()
            .filter_map(|tech| {
                if let Tech::UnitTech(unit) = tech {
                    Some(*unit)
                } else {
                    None
                }
            })
            .chain(Unit::BASIC_UNITS.into_iter())
            .collect();

        // Randomly buy units
        while money > 0 {
            let affordable: Vec<_> = available_units
                .iter()
                .filter(|u| u.stats().cost <= money)
                .collect();
            if affordable.is_empty() || rng.gen_bool(0.2) {
                // 20% chance to stop buying
                break;
            }
            let unit = affordable.choose(rng).unwrap();
            actions.push(SpawnAction::Buy { unit: **unit });
            money -= unit.stats().cost;
        }

        let mut all_units_to_spawn: Vec<_> = board.reinforcements[side].iter().cloned().collect();
        for action in &actions {
            if let SpawnAction::Buy { unit } = action {
                all_units_to_spawn.push(*unit);
            }
        }

        let mut all_spawn_locs = board.bitboards.get_spawn_locs(side, true);
        let mut land_spawn_locs = board.bitboards.get_spawn_locs(side, false);

        all_units_to_spawn.shuffle(rng);

        for unit in all_units_to_spawn {
            if unit.stats().flying {
                if let Some(loc) = all_spawn_locs.pop() {
                    actions.push(SpawnAction::Spawn {
                        spawn_loc: loc,
                        unit,
                    });
                    land_spawn_locs.set(loc, false);
                }
            } else {
                if let Some(loc) = land_spawn_locs.pop() {
                    actions.push(SpawnAction::Spawn {
                        spawn_loc: loc,
                        unit,
                    });
                    all_spawn_locs.set(loc, false);
                }
            }
        }

        actions
    }
}
