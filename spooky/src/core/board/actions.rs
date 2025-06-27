use crate::core::{
    board::{definitions::Phase, BitboardOps, Board}, loc::Loc, side::Side, spells::{Spell, SpellCast}, units::Unit
};
use anyhow::{bail, ensure, Result, Context};
use std::fmt::Display;
use std::str::FromStr;

/// Actions taken during the setup phase of a board reset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupAction {
    pub necromancer_choice: Unit,
    pub saved_unit: Option<Unit>,
}

/// Actions taken during the attack phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttackAction {
    Move {
        from_loc: Loc,
        to_loc: Loc,
    },
    MoveCyclic {
        locs: Vec<Loc>, // locs[i] moves to locs[(i + 1) % locs.len()]
    },
    Attack {
        attacker_loc: Loc,
        target_loc: Loc,
    },
    Blink {
        blink_loc: Loc,
    },
    Cast {
        spell_cast: SpellCast,
    },
    Resign,
}

/// Actions taken during the spawn phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnAction {
    Buy {
        unit: Unit,
    },
    Spawn {
        spawn_loc: Loc,
        unit: Unit,
    },
    Discard {
        spell: Spell,
    },
}

/// Represents the actions taken during a single board's turn phase.
#[derive(Debug, Default, Clone)]
pub struct BoardTurn {
    pub setup_action: Option<SetupAction>,
    pub attack_actions: Vec<AttackAction>,
    pub spawn_actions: Vec<SpawnAction>,
}

impl Display for SetupAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "setup {}", self.necromancer_choice)?;

        if let Some(saved_unit) = &self.saved_unit {
            write!(f, " {}", saved_unit)?;
        }

        Ok(())
    }
}

impl SetupAction {
    pub fn from_args(action_name: &str, args: &[&str]) -> Result<Self> {
        match action_name {
            "setup" => {
                ensure!(args.len() == 1 || args.len() == 2, "setup requires 1 or 2 arguments");
                let necromancer_choice = args[0].parse()?;
                let saved_unit = if args.len() == 2 {
                    Some(args[1].parse()?)
                } else {
                    None
                };
                Ok(SetupAction { necromancer_choice, saved_unit })
            }
            _ => bail!("Unknown setup action: {}", action_name),
        }
    }
}

impl Display for AttackAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttackAction::Move { from_loc, to_loc } => write!(f, "move {} {}", from_loc, to_loc),
            AttackAction::MoveCyclic { locs } => {
                let locs_str: Vec<String> = locs.iter().map(|l| l.to_string()).collect();
                write!(f, "move_cyclic {}", locs_str.join(" "))
            }
            AttackAction::Attack {
                attacker_loc,
                target_loc,
            } => write!(f, "attack {} {}", attacker_loc, target_loc),
            AttackAction::Blink { blink_loc } => write!(f, "blink {}", blink_loc),
            AttackAction::Cast { spell_cast } => write!(f, "cast {}", spell_cast),
            AttackAction::Resign => write!(f, "resign"),
        }
    }
}

impl AttackAction {
    pub fn from_args(action_name: &str, args: &[&str]) -> Result<Self> {
        match action_name {
            "move" => {
                ensure!(args.len() == 2, "move requires 2 arguments");
                let from_loc = args[0].parse()?;
                let to_loc = args[1].parse()?;
                Ok(AttackAction::Move { from_loc, to_loc })
            }
            "move_cyclic" => {
                let locs = args
                    .iter()
                    .map(|s| s.parse())
                    .collect::<Result<Vec<_>>>()?;
                Ok(AttackAction::MoveCyclic { locs })
            }
            "attack" => {
                ensure!(args.len() == 2, "attack requires 2 arguments");
                let attacker_loc = args[0].parse()?;
                let target_loc = args[1].parse()?;
                Ok(AttackAction::Attack {
                    attacker_loc,
                    target_loc,
                })
            }
            "blink" => {
                ensure!(args.len() == 1, "blink requires 1 argument");
                let blink_loc = args[0].parse()?;
                Ok(AttackAction::Blink { blink_loc })
            }
            "cast" => {
                let spell_cast = SpellCast::from_str(&args.join(" "))?;
                Ok(AttackAction::Cast { spell_cast })
            }
            "resign" => Ok(AttackAction::Resign),
            _ => bail!("Unknown attack action: {}", action_name),
        }
    }
}

impl Display for SpawnAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpawnAction::Buy { unit } => write!(f, "buy {}", unit),
            SpawnAction::Spawn { spawn_loc, unit } => write!(f, "spawn {} {}", unit, spawn_loc),
            SpawnAction::Discard { spell } => write!(f, "discard {}", spell),
        }
    }
}

impl SpawnAction {
    pub fn from_args(action_name: &str, args: &[&str]) -> Result<Self> {
        match action_name {
            "buy" => {
                ensure!(args.len() == 1, "buy requires 1 argument");
                let unit = args[0].parse()?;
                Ok(SpawnAction::Buy { unit })
            }
            "spawn" => {
                ensure!(args.len() == 2, "spawn requires 2 arguments");
                let unit = args[0].parse()?;
                let spawn_loc = args[1].parse()?;
                Ok(SpawnAction::Spawn { unit, spawn_loc })
            }
            "discard" => {
                ensure!(args.len() == 1, "discard requires 1 argument");
                let spell = args[0].parse()?;
                Ok(SpawnAction::Discard { spell })
            }
            _ => bail!("Unknown spawn action: {}", action_name),
        }
    }
}

impl<'a> Board<'a> {
    pub fn do_setup_action(&mut self, side: Side, action: SetupAction) -> Result<()> {
        ensure!(self.state.phases().contains(&Phase::Setup), "invalid setup phase");

        let necromancer_unit = action.necromancer_choice;
        ensure!(necromancer_unit.stats().necromancer, "Cannot choose non-necromancer unit as necromancer");

        let necromancer_loc = Board::NECROMANCER_START_LOC[side];
        self.spawn_piece(side, necromancer_loc, necromancer_unit)?;

        if let Some(saved_unit) = action.saved_unit {
            self.add_reinforcement(saved_unit, side);
        }

        self.add_reinforcement(Unit::Initiate, side);
        
        Ok(())
    }

    // returns rebate
    pub fn do_attack_action(
        &mut self,
        side: Side,
        action: AttackAction,
    ) -> Result<i32> {
        ensure!(self.state.phases().contains(&Phase::Attack), "invalid attack phase");

        match action {
            AttackAction::Move { from_loc, to_loc } => {
                let piece = self.get_piece(&from_loc).context("No piece to move")?;
                if piece.side != side {
                    bail!("Cannot move opponent's piece");
                }
                if !piece.state.can_act() {
                    println!("{:#?}", self);
                    bail!("Piece has already moved or attacked");
                }
                if self.get_piece(&to_loc).is_ok() {
                    bail!("Destination square is occupied");
                }

                let mut piece_to_move = self.remove_piece(&from_loc)
                    .expect("Piece should exist here");
                piece_to_move.state.moved = true;
                piece_to_move.loc = to_loc;
                self.add_piece(piece_to_move);

                Ok(0)
            }
            AttackAction::MoveCyclic { locs } => {
                self.move_pieces_cyclic(side, &locs)?;
                Ok(0)
            }
            AttackAction::Attack {
                attacker_loc,
                target_loc,
            } => {
                self.attack_piece(attacker_loc, target_loc, side)
            }
            AttackAction::Blink { blink_loc } => {
                let piece = self.get_piece(&blink_loc).context("No piece to blink")?.clone();
                if piece.side != side {
                    bail!("Cannot blink opponent's piece");
                }
                if !piece.unit.stats().blink {
                    bail!("Piece cannot blink");
                }
                if !piece.state.can_act() {
                    bail!("Blinking piece has already moved or attacked");
                }

                self.bounce_piece(piece);

                Ok(0)
            }
            AttackAction::Cast { spell_cast } => {
                spell_cast.cast(self, side)?;
                Ok(0)
            }
            AttackAction::Resign => {
                self.resign(side);
                Ok(0)
            }
        }
    }

    // returns total rebate
    pub fn do_attacks(&mut self, side: Side, attack_actions: &[AttackAction]) -> Result<i32> {
        let mut rebate = 0;
        for action in attack_actions {
            let amount = self.do_attack_action(side, action.clone())?;
            rebate += amount;
        }
        Ok(rebate)
    }

    // mutates money
    pub fn do_spawn_action(&mut self, side: Side, money: &mut i32, action: SpawnAction) -> Result<()> {
        ensure!(self.state.phases().contains(&Phase::Spawn), "invalid spawn phase");

        match action {
            SpawnAction::Buy { unit } => {
                let cost = unit.stats().cost;
                if *money < cost {
                    bail!("Not enough money to buy unit");
                }
                *money -= cost;
                self.reinforcements[side].insert(unit);
            }
            SpawnAction::Spawn { spawn_loc, unit } => {
                if !self.bitboards.get_spawn_locs(side, unit.stats().flying).get(spawn_loc) {
                    bail!("Can only spawn units on keeps");
                }
                if self.get_piece(&spawn_loc).is_ok() {
                    bail!("Cannot spawn on occupied location");
                }
                if self.reinforcements[side].remove(&unit) == 0 {
                    bail!("Unit not in reinforcements");
                }

                self.spawn_piece(side, spawn_loc, unit)?;
            }
            SpawnAction::Discard { .. } => todo!(),
        }

        Ok(())
    }
     
    // returns money left after spawns
    pub fn do_spawns(&mut self, side: Side, money: i32, spawn_actions: &[SpawnAction]) -> Result<i32> {
        let mut money = money;
        for action in spawn_actions {
            self.do_spawn_action(side, &mut money, action.clone())?;
        }
        Ok(money)
    }
}