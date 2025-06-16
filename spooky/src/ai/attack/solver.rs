use std::collections::{HashMap, HashSet};
use crate::core::{Loc, Side, units::Attack};
use super::{combat::CombatGraph, prophecy::Prophecy};

/// Represents the state of a piece in combat
#[derive(Debug, Clone)]
pub struct CombatState {
    pub passive: bool,
    pub removed: bool,
    pub attack_hex: Option<Loc>,
    pub targets: Vec<Loc>,
    pub damage_dealt: i32,
}

impl CombatState {
    pub fn new() -> Self {
        Self {
            passive: false,
            removed: false,
            attack_hex: None,
            targets: Vec::new(),
            damage_dealt: 0,
        }
    }
}

/// Basic constraint solver for combat resolution
pub struct CombatSolver {
    pub graph: CombatGraph,
    pub prophecy: Prophecy,
    pub attacker_states: HashMap<Loc, CombatState>,
    pub defender_states: HashMap<Loc, CombatState>,
    pub hex_assignments: HashMap<Loc, Loc>, // hex -> attacker
    pub threshold: f32,
}

impl CombatSolver {
    pub fn new(graph: CombatGraph, prophecy: Prophecy) -> Self {
        let mut attacker_states = HashMap::new();
        let mut defender_states = HashMap::new();

        // Initialize states
        for attacker in &graph.attackers {
            attacker_states.insert(*attacker, CombatState::new());
        }
        for defender in &graph.defenders {
            defender_states.insert(*defender, CombatState::new());
        }

        Self {
            graph,
            prophecy,
            attacker_states,
            defender_states,
            hex_assignments: HashMap::new(),
            threshold: 0.5,
        }
    }

    /// Solve the combat constraints based on the prophecy
    pub fn solve(&mut self) -> bool {
        // Sort pieces by probability
        let mut sorted_attackers: Vec<_> = self.graph.attackers.iter()
            .filter_map(|loc| {
                self.prophecy.passive_probabilities.get(loc)
                    .map(|prob| (*loc, *prob))
            })
            .collect();
        sorted_attackers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let mut sorted_defenders: Vec<_> = self.graph.defenders.iter()
            .filter_map(|loc| {
                self.prophecy.removal_probabilities.get(loc)
                    .map(|prob| (*loc, *prob))
            })
            .collect();
        sorted_defenders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Apply constraints in probability order
        for (attacker_loc, prob) in sorted_attackers {
            if prob > self.threshold {
                if !self.try_make_passive(attacker_loc) {
                    // If we can't make it passive, try to make it active
                    if !self.try_make_active(attacker_loc) {
                        return false; // UNSAT
                    }
                }
            }
        }

        for (defender_loc, prob) in sorted_defenders {
            if prob > self.threshold {
                if !self.try_remove_defender(defender_loc) {
                    // If we can't remove it, leave it
                    if !self.try_keep_defender(defender_loc) {
                        return false; // UNSAT
                    }
                }
            }
        }

        true // SAT
    }

    fn try_make_passive(&mut self, attacker_loc: Loc) -> bool {
        let state = self.attacker_states.get_mut(&attacker_loc).unwrap();
        state.passive = true;
        true
    }

    fn try_make_active(&mut self, attacker_loc: Loc) -> bool {
        let state = self.attacker_states.get_mut(&attacker_loc).unwrap();
        state.passive = false;

        // Find a valid attack hex
        if let Some(attack_hexes) = self.graph.attack_hexes.get(&attacker_loc) {
            for hex in attack_hexes {
                if !self.hex_assignments.contains_key(hex) {
                    self.hex_assignments.insert(*hex, attacker_loc);
                    state.attack_hex = Some(*hex);

                    // Find targets from this hex
                    for pair in &self.graph.pairs {
                        if pair.attacker_pos == attacker_loc && pair.attack_hexes.contains(hex) {
                            state.targets.push(pair.defender_pos);
                        }
                    }
                    return true;
                }
            }
        }

        false
    }

    fn try_remove_defender(&mut self, defender_loc: Loc) -> bool {
        let state = self.defender_states.get_mut(&defender_loc).unwrap();
        state.removed = true;
        true
    }

    fn try_keep_defender(&mut self, defender_loc: Loc) -> bool {
        let state = self.defender_states.get_mut(&defender_loc).unwrap();
        state.removed = false;
        true
    }

    /// Generate a move from the solved state
    pub fn generate_move(&self, board: &crate::core::Board, side: Side) -> Vec<crate::core::action::BoardAction> {
        let mut actions = Vec::new();

        // Generate movement actions
        for (attacker_loc, state) in &self.attacker_states {
            if !state.passive {
                if let Some(attack_hex) = state.attack_hex {
                    if *attacker_loc != attack_hex {
                        actions.push(crate::core::action::BoardAction::Move {
                            from_loc: *attacker_loc,
                            to_loc: attack_hex,
                        });
                    }
                }
            }
        }

        // Generate attack actions
        for (attacker_loc, state) in &self.attacker_states {
            if !state.passive {
                for target in &state.targets {
                    if let Some(attack_hex) = state.attack_hex {
                        actions.push(crate::core::action::BoardAction::Attack {
                            attacker_loc: attack_hex,
                            target_loc: *target,
                        });
                    }
                }
            }
        }

        actions
    }

    /// Check if the generated move is valid
    pub fn is_valid_move(&self, board: &crate::core::Board, side: Side) -> bool {
        // Basic validation - check that all pieces exist and are on the correct side
        for (attacker_loc, state) in &self.attacker_states {
            if !state.passive {
                if let Some(piece) = board.get_piece(attacker_loc) {
                    if piece.side != side {
                        return false;
                    }
                } else {
                    return false;
                }

                for target in &state.targets {
                    if let Some(piece) = board.get_piece(target) {
                        if piece.side == side {
                            return false; // Can't attack own piece
                        }
                    } else {
                        return false;
                    }
                }
            }
        }

        true
    }
}