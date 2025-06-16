use std::collections::HashMap;
use crate::core::{Loc, Side, units::{Attack, UnitStats}};
use super::combat::CombatGraph;

/// Variables for the constraint satisfaction problem
#[derive(Debug, Clone)]
pub struct Variables {
    // Defender variables
    pub removed: HashMap<Loc, bool>,      // r_y: whether piece is removed
    pub removal_time: HashMap<Loc, i32>,  // tr_y: time of removal
    pub killed: HashMap<Loc, bool>,       // k_y: whether piece is killed
    pub unsummoned: HashMap<Loc, bool>,   // u_y: whether piece is unsummoned

    // Attacker variables
    pub passive: HashMap<Loc, bool>,      // p_x: whether piece is passive
    pub attack_time: HashMap<Loc, i32>,   // ta_x: combat engagement time
    pub attack_hex: HashMap<Loc, Option<Loc>>, // m_xs: which hex attacker moved to

    // Combat variables
    pub attacks: HashMap<(Loc, Loc), bool>, // a_xy: whether x attacked y
    pub damage: HashMap<(Loc, Loc), i32>,   // d_xy: damage dealt by x to y
}

impl Variables {
    pub fn new(graph: &CombatGraph) -> Self {
        let mut vars = Variables {
            removed: HashMap::new(),
            removal_time: HashMap::new(),
            killed: HashMap::new(),
            unsummoned: HashMap::new(),
            passive: HashMap::new(),
            attack_time: HashMap::new(),
            attack_hex: HashMap::new(),
            attacks: HashMap::new(),
            damage: HashMap::new(),
        };

        // Initialize defender variables
        for defender in &graph.defenders {
            vars.removed.insert(*defender, false);
            vars.removal_time.insert(*defender, 0);
            vars.killed.insert(*defender, false);
            vars.unsummoned.insert(*defender, false);
        }

        // Initialize attacker variables
        for attacker in &graph.attackers {
            vars.passive.insert(*attacker, false);
            vars.attack_time.insert(*attacker, 0);
            vars.attack_hex.insert(*attacker, None);
        }

        // Initialize combat variables
        for pair in &graph.pairs {
            vars.attacks.insert((pair.attacker_pos, pair.defender_pos), false);
            vars.damage.insert((pair.attacker_pos, pair.defender_pos), 0);
        }

        vars
    }
}

/// Constraint types for the combat system
#[derive(Debug, Clone)]
pub enum Constraint {
    // Hex constraints
    AtMostOnePiecePerHex(Loc),
    MovementTiming(Loc, Loc, i32), // hex, attacker, time

    // Attacker constraints
    ExactlyOneAction(Loc), // passive OR move to exactly one hex
    AttackRange(Loc, Loc), // attacker, defender
    AttackTiming(Loc, Loc), // attacker, defender
    AttackLimit(Loc, i32), // attacker, max attacks
    DamageCalculation(Loc, Loc), // attacker, defender

    // Defender constraints
    ExactlyOneFate(Loc), // not removed OR killed OR unsummoned
    UnsummonCondition(Loc), // unsummoned if attacked by unsummon
    DamageResolution(Loc), // removed if damage >= defense
    NoOverkill(Loc), // optional: no overkill
}

/// Constraint solver for combat resolution
pub struct ConstraintSolver {
    pub variables: Variables,
    pub constraints: Vec<Constraint>,
    pub hex_assignments: HashMap<Loc, Loc>, // hex -> attacker
    pub time_counter: i32,
}

impl ConstraintSolver {
    pub fn new(graph: &CombatGraph) -> Self {
        Self {
            variables: Variables::new(graph),
            constraints: Vec::new(),
            hex_assignments: HashMap::new(),
            time_counter: 0,
        }
    }

    /// Add all constraints for the combat graph
    pub fn add_all_constraints(&mut self, graph: &CombatGraph, board: &crate::core::Board) {
        // Hex constraints
        for hex in &graph.hexes {
            self.constraints.push(Constraint::AtMostOnePiecePerHex(*hex));
        }

        // Attacker constraints
        for attacker in &graph.attackers {
            self.constraints.push(Constraint::ExactlyOneAction(*attacker));

            if let Some(piece) = board.get_piece(attacker) {
                let stats = piece.unit.stats();
                self.constraints.push(Constraint::AttackLimit(*attacker, stats.num_attacks));
            }
        }

        // Combat pair constraints
        for pair in &graph.pairs {
            self.constraints.push(Constraint::AttackRange(pair.attacker_pos, pair.defender_pos));
            self.constraints.push(Constraint::AttackTiming(pair.attacker_pos, pair.defender_pos));
            self.constraints.push(Constraint::DamageCalculation(pair.attacker_pos, pair.defender_pos));
        }

        // Defender constraints
        for defender in &graph.defenders {
            self.constraints.push(Constraint::ExactlyOneFate(*defender));
            self.constraints.push(Constraint::UnsummonCondition(*defender));
            self.constraints.push(Constraint::DamageResolution(*defender));
        }
    }

    /// Check if a constraint is satisfied
    pub fn check_constraint(&self, constraint: &Constraint, board: &crate::core::Board) -> bool {
        match constraint {
            Constraint::AtMostOnePiecePerHex(hex) => {
                let mut count = 0;
                for (attacker, hex_assignment) in &self.variables.attack_hex {
                    if let Some(assigned_hex) = hex_assignment {
                        if assigned_hex == hex {
                            count += 1;
                        }
                    }
                }
                count <= 1
            }

            Constraint::ExactlyOneAction(attacker) => {
                let is_passive = self.variables.passive.get(attacker).unwrap_or(&false);
                let has_hex = self.variables.attack_hex.get(attacker).unwrap_or(&None).is_some();
                *is_passive != has_hex // exactly one must be true
            }

            Constraint::AttackRange(attacker, defender) => {
                if let Some(attack_hex) = self.variables.attack_hex.get(attacker).unwrap_or(&None) {
                    if let Some(piece) = board.get_piece(attacker) {
                        let stats = piece.unit.stats();
                        attack_hex.dist(defender) <= stats.range
                    } else {
                        false
                    }
                } else {
                    true // no attack hex means no attack
                }
            }

            Constraint::AttackTiming(attacker, defender) => {
                let attack_time = self.variables.attack_time.get(attacker).unwrap_or(&0);
                let removal_time = self.variables.removal_time.get(defender).unwrap_or(&0);
                *attack_time <= *removal_time
            }

            Constraint::AttackLimit(attacker, max_attacks) => {
                let mut attack_count = 0;
                for (_, is_attack) in &self.variables.attacks {
                    if *is_attack {
                        attack_count += 1;
                    }
                }
                attack_count <= *max_attacks
            }

            Constraint::DamageCalculation(attacker, defender) => {
                if let Some(is_attack) = self.variables.attacks.get(&(*attacker, *defender)) {
                    if !*is_attack {
                        return true; // no attack, no damage
                    }

                    if let Some(attacker_piece) = board.get_piece(attacker) {
                        if let Some(defender_piece) = board.get_piece(defender) {
                            let attacker_stats = attacker_piece.unit.stats();
                            let defender_stats = defender_piece.unit.stats();

                            let expected_damage = match attacker_stats.attack {
                                Attack::Damage(damage) => damage,
                                Attack::Deathtouch => {
                                    if defender_stats.necromancer {
                                        0
                                    } else {
                                        defender_stats.defense
                                    }
                                }
                                Attack::Unsummon => {
                                    if defender_stats.persistent {
                                        1
                                    } else {
                                        0 // unsummon doesn't do damage
                                    }
                                }
                            };

                            let actual_damage = self.variables.damage.get(&(*attacker, *defender)).unwrap_or(&0);
                            *actual_damage == expected_damage
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    true
                }
            }

            Constraint::ExactlyOneFate(defender) => {
                let removed = self.variables.removed.get(defender).unwrap_or(&false);
                let killed = self.variables.killed.get(defender).unwrap_or(&false);
                let unsummoned = self.variables.unsummoned.get(defender).unwrap_or(&false);

                // Exactly one of removed, killed, or unsummoned must be true
                (*removed as i32 + *killed as i32 + *unsummoned as i32) == 1
            }

            Constraint::UnsummonCondition(defender) => {
                let unsummoned = self.variables.unsummoned.get(defender).unwrap_or(&false);
                if !*unsummoned {
                    return true; // not unsummoned, constraint satisfied
                }

                // Check if any attacker with unsummon attack is attacking this defender
                let mut has_unsummon_attacker = false;
                for ((attacker, defender_check), is_attack) in &self.variables.attacks {
                    if defender_check == defender && *is_attack {
                        if let Some(piece) = board.get_piece(attacker) {
                            if matches!(piece.unit.stats().attack, Attack::Unsummon) {
                                has_unsummon_attacker = true;
                                break;
                            }
                        }
                    }
                }

                has_unsummon_attacker
            }

            Constraint::DamageResolution(defender) => {
                let removed = self.variables.removed.get(defender).unwrap_or(&false);
                if !*removed {
                    return true; // not removed, constraint satisfied
                }

                // Calculate total damage
                let mut total_damage = 0;
                for ((attacker, defender_check), damage) in &self.variables.damage {
                    if defender_check == defender {
                        total_damage += damage;
                    }
                }

                if let Some(piece) = board.get_piece(defender) {
                    let stats = piece.unit.stats();
                    total_damage >= stats.defense
                } else {
                    false
                }
            }

            _ => true, // Other constraints not fully implemented yet
        }
    }

    /// Check if all constraints are satisfied
    pub fn is_satisfiable(&self, board: &crate::core::Board) -> bool {
        for constraint in &self.constraints {
            if !self.check_constraint(constraint, board) {
                return false;
            }
        }
        true
    }

    /// Apply a prophecy to the variables
    pub fn apply_prophecy(&mut self, prophecy: &super::prophecy::Prophecy) {
        // Apply passive probabilities
        for (loc, prob) in &prophecy.passive_probabilities {
            if let Some(passive) = self.variables.passive.get_mut(loc) {
                *passive = *prob > 0.5;
            }
        }

        // Apply removal probabilities
        for (loc, prob) in &prophecy.removal_probabilities {
            if let Some(removed) = self.variables.removed.get_mut(loc) {
                *removed = *prob > 0.5;
            }
        }
    }

    /// Generate a move from the solved variables
    pub fn generate_move(&self, board: &crate::core::Board, side: Side) -> Vec<crate::core::action::BoardAction> {
        let mut actions = Vec::new();

        // Generate movement actions
        for (attacker, attack_hex) in &self.variables.attack_hex {
            if let Some(hex) = attack_hex {
                if let Some(piece) = board.get_piece(attacker) {
                    if piece.side == side && *attacker != *hex {
                        actions.push(crate::core::action::BoardAction::Move {
                            from_loc: *attacker,
                            to_loc: *hex,
                        });
                    }
                }
            }
        }

        // Generate attack actions
        for ((attacker, defender), is_attack) in &self.variables.attacks {
            if *is_attack {
                if let Some(attack_hex) = self.variables.attack_hex.get(attacker) {
                    if let Some(hex) = attack_hex {
                        actions.push(crate::core::action::BoardAction::Attack {
                            attacker_loc: *hex,
                            target_loc: *defender,
                        });
                    }
                }
            }
        }

        actions
    }
}