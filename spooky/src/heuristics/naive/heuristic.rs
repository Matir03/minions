use crate::{
    ai::Eval,
    core::{
        board::actions::BoardTurn,
        tech::{TechAssignment, TechState, Techline},
        Board, GameState, Map, GameConfig, Side,
        utils::Sigmoid,
        tech::SPELL_COST,
        SideArray,
    },
    heuristics::traits::{Heuristic, LocalHeuristic, TechlineHeuristic, BoardHeuristic},
};

use super::{
    board::{BoardAcc, BoardPre},
    techline::{TechAcc, TechPre, NaiveShared},
};

#[derive(Debug)]
pub struct NaiveHeuristic {
    pub techline: Techline,
    pub map: Map,
}

// Implement LocalHeuristic for Techline
impl<'a> LocalHeuristic<&'a Techline, TechState, TechAssignment, NaiveShared> for NaiveHeuristic {
    type Acc = TechAcc;
    type Pre = TechPre;

    fn new(config: &'a Techline) -> Self {
        Self {
            techline: config.clone(),
            map: Map::BlackenedShores, // Default map
        }
    }

    fn compute_acc(&self, state: &TechState) -> Self::Acc {
        TechAcc {
            tech_state: state.clone(),
            side: Side::Yellow,
        }
    }

    fn update_acc(&self, acc: &Self::Acc, turn: &TechAssignment) -> Self::Acc {
        let mut new_tech_state = acc.tech_state.clone();
        new_tech_state
            .assign_techs(turn.clone(), acc.side, &self.techline)
            .expect("Failed to assign techs");

        TechAcc {
            tech_state: new_tech_state,
            side: !acc.side,
        }
    }

    fn compute_pre(&self, _state: &TechState, _acc: &Self::Acc) -> Self::Pre {
        TechPre { max_techs: 0 }
    }

    fn compute_turn(&self, blotto: i32, shared: &NaiveShared, _pre: &Self::Pre) -> TechAssignment {
        use std::collections::HashSet;
        
        let spell_cost = shared.config.spell_cost();
        let mut advance_by = 0;
        
        if blotto >= spell_cost {
            advance_by = 1;
        }
        
        TechAssignment::new(advance_by, Vec::new())
    }
}

// Implement LocalHeuristic for Board
impl<'a> LocalHeuristic<&'a Map, Board<'a>, BoardTurn, NaiveShared> for NaiveHeuristic {
    type Acc = BoardAcc;
    type Pre = BoardPre;

    fn new(config: &'a Map) -> Self {
        Self {
            techline: Techline::default(),
            map: *config,
        }
    }

    fn compute_acc(&self, state: &Board<'a>) -> Self::Acc {
        let static_board = unsafe { std::mem::transmute::<Board<'a>, Board<'static>>(state.clone()) };
        
        BoardAcc {
            board: static_board,
            side_to_move: Side::Yellow,
            delta_money: SideArray::new(0, 0),
            delta_points: SideArray::new(0, 0),
        }
    }

    fn update_acc(&self, acc: &Self::Acc, _turn: &BoardTurn) -> Self::Acc {
        let mut new_acc = acc.clone();
        new_acc.side_to_move = !acc.side_to_move;
        new_acc
    }

    fn compute_pre(&self, _state: &Board<'a>, _acc: &Self::Acc) -> Self::Pre {
        BoardPre {
            setup_action: None,
            resign_available: true,
        }
    }

    fn compute_turn(&self, blotto: i32, _shared: &NaiveShared, pre: &Self::Pre) -> BoardTurn {
        if blotto <= 0 {
            return BoardTurn::Resign;
        }
        
        // Simple turn generation for now
        if let Some(setup_action) = &pre.setup_action {
            use crate::core::board::actions::BoardActions;
            BoardTurn::Actions(BoardActions {
                setup_action: Some(setup_action.clone()),
                attack_actions: vec![],
                spawn_actions: vec![],
            })
        } else {
            BoardTurn::Resign
        }
    }
}

// Implement the marker traits
impl<'a> TechlineHeuristic<'a, NaiveShared> for NaiveHeuristic {}
impl<'a> BoardHeuristic<'a, NaiveShared> for NaiveHeuristic {}

impl<'a> Heuristic<'a> for NaiveHeuristic {
    type Shared = NaiveShared;

    fn compute_shared(
        &self,
        game_state: &GameState<'a>,
        _general: &<Self as LocalHeuristic<&'a Techline, TechState, TechAssignment, Self::Shared>>::Acc,
        _boards: &[&<Self as LocalHeuristic<&'a Map, Board<'a>, BoardTurn, Self::Shared>>::Acc],
    ) -> Self::Shared {
        NaiveShared {
            config: game_state.config.clone(),
        }
    }

    fn compute_blottos(&self, shared: &Self::Shared) -> Vec<Vec<i32>> {
        // Simple money distribution heuristic
        // This is a simplified version of the blotto logic from ai::blotto::distribute_money
        let num_boards = shared.config.num_boards;
        
        vec![
            vec![10; num_boards], // Simple equal distribution
            vec![8; num_boards],  // Alternative distribution
        ]
    }

    fn compute_eval(&self, shared: &Self::Shared) -> Eval {
        // Return a neutral evaluation for now
        // In practice, this should compute based on the shared game state
        Eval::new(0.5, Side::Yellow)
    }
}

impl NaiveHeuristic {
    pub fn new(config: &GameConfig) -> Self {
        Self {
            techline: config.techline.clone(),
            map: config.maps[0], // Use first map for simplicity
        }
    }
    
    /// Heuristic evaluation extracted from ai::eval::Eval::heuristic
    pub fn heuristic_eval(config: &GameConfig, state: &GameState) -> Eval {
        const BOARD_POINT_VALUE: i32 = 30;
        const SIGMOID_SCALE: f32 = 0.01;
        let have_the_move_bonus = 10 * config.num_boards as i32;

        let mut dollar_diff = 0;

        // Dollars on boards
        for board in state.boards.iter() {
            for piece in board.pieces.values() {
                let value = piece.unit.stats().cost;
                dollar_diff += value * piece.side.sign();
            }
        }

        for side in Side::all() {
            // Dollars on techline
            let num_techs = state.tech_state.acquired_techs[side].len() as i32;
            dollar_diff += num_techs * SPELL_COST * config.num_boards as i32 * side.sign();

            // Money in hand
            dollar_diff += state.money[side] * side.sign();

            // Board points value
            dollar_diff += state.board_points[side] * BOARD_POINT_VALUE * side.sign();
        }

        // Add a small bonus for the side to move
        dollar_diff += have_the_move_bonus * state.side_to_move.sign();

        // The dollar_diff is from Yellow's perspective. We need to adjust for side_to_move.
        let perspective_diff = dollar_diff * state.side_to_move.sign();

        // Convert to winprob
        let winprob = (perspective_diff as f32 * SIGMOID_SCALE).sigmoid();

        Eval::new(winprob, state.side_to_move)
    }
    
    /// Simple money distribution heuristic extracted from ai::blotto::distribute_money
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
}