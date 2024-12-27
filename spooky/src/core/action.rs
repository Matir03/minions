//! Game actions and moves

use super::{
    loc::Loc,
    units::Unit,
    spells::{Spell, SpellCast},
    tech::TechAssignment
};

use anyhow::{ensure, Result, bail};
use hashbag::HashBag;
use std::ops::{Range, RangeBounds};

/// Represents a legal move in the game
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardAction {
    Move {
        from_loc: Loc,
        to_loc: Loc,
    },
    MoveCyclic {
        locs: Vec<Loc>,
    },
    Attack {
        attacker_loc: Loc,
        target_loc: Loc,
    },
    Blink {
        blink_loc: Loc,
    },
    Buy {
        unit: Unit,
    },
    Spawn {
        spawn_loc: Loc,
        unit: Unit,
    },
    Cast {
        spell_cast: SpellCast,
    },
    Discard {
        spell: Spell,
    },
    EndPhase,
    Resign,
    SaveUnit {
        unit: Option<Unit>,
    }
}

impl BoardAction {
    pub fn from_args(name: &str, args: &[&str]) -> Result<BoardAction> {
        let expected_numargs = match name {
            "move" => 2..=2,
            "movecyclic" => 1..=100,
            "attack" => 2..=2,
            "blink" => 1..=1,
            "buy" => 1..=1,
            "spawn" => 2..=2,
            "cast" => 1..=10,
            "discard" => 1..=1,
            "endphase" => 0..=0,
            "resign" => 0..=0,
            "saveunit" => 0..=1,
            _ => bail!("Invalid board action name"),
        };

        let num_args = args.len();
        ensure!(expected_numargs.contains(&num_args), "Invalid number of arguments");

        let action = match name {
            "move" => BoardAction::Move {
                from_loc: args[0].parse()?, 
                to_loc: args[1].parse()?
            },
            "movecyclic" => BoardAction::MoveCyclic {
                locs: args.iter().map(|s| s.parse()).collect::<Result<Vec<_>>>()?,
            },
            "attack" => BoardAction::Attack {
                attacker_loc: args[0].parse()?, 
                target_loc: args[1].parse()?
            },
            "blink" => BoardAction::Blink {
                blink_loc: args[0].parse()?,
            },
            "buy" => BoardAction::Buy {
                unit: args[0].parse()?,
            },
            "spawn" => BoardAction::Spawn {
                spawn_loc: args[0].parse()?, 
                unit: args[1].parse()?
            },
            "cast" => todo!("spells not implemented yet"),
            "discard" => BoardAction::Discard {
                spell: args[0].parse()?,
            },
            "endphase" => BoardAction::EndPhase,
            "resign" => BoardAction::Resign,
            "saveunit" => BoardAction::SaveUnit {
                unit: args.get(0).map(|s| s.parse()).transpose()?,
            },
            _ => unreachable!(),
        };

        Ok(action)
    }
}
pub enum GameAction {
    BuySpell(Spell),
    AdvanceTech(usize),
    AcquireTech(usize),
    GiveSpell(usize, Spell),
    BoardAction(usize, BoardAction),
}

impl GameAction {
    pub fn from_args(name: &str, args: &[&str]) -> Result<GameAction> {
        let expected_numargs = match name {
            "buyspell" => 1..=1,
            "advancetech" => 1..=1,
            "acquiretech" => 1..=1,
            "givespell" => 2..=2,
            "boardaction" => 2..=100,
            _ => bail!("Invalid board action name"),
        };

        let num_args = args.len();
        ensure!(expected_numargs.contains(&num_args), "Invalid number of arguments");

        let action = match name {
            "buyspell" => GameAction::BuySpell(args[0].parse()?),
            "advancetech" => GameAction::AdvanceTech(args[0].parse()?),
            "acquiretech" => GameAction::AcquireTech(args[0].parse()?),
            "givespell" => GameAction::GiveSpell(args[0].parse()?, args[1].parse()?),
            "boardaction" => GameAction::BoardAction(args[0].parse()?, 
                BoardAction::from_args(args[1], &args[2..])?),
            _ => unreachable!(),
        };

        Ok(action)
    }
}

/// Represents a complete turn in the game
#[derive(Debug, Clone)]
pub struct Turn {
    // pub num_spells_bought: usize,
    pub spells: HashBag<Spell>,
    pub spell_assignment: Vec<Spell>,  // board -> spell index
    pub tech_assignment: TechAssignment,
    pub board_actions: Vec<Vec<BoardAction>>,
}

impl Turn {
    /// Create a new turn with the given number of boards
    pub fn new(spells: Vec<Spell>, num_boards: usize) -> Self {
        assert!(spells.len() == num_boards + 1, "Invalid number of spells");

        Self {
            // num_spells_bought: 0,
            spells: spells.into_iter().collect(),
            spell_assignment: vec![Spell::Blank; num_boards],
            tech_assignment: TechAssignment::default(),
            board_actions: vec![Vec::new(); num_boards],
        }
    }

    pub fn do_action(&mut self, action: GameAction) -> Result<()> {
        match action {
            GameAction::BuySpell(spell) => {
                self.spells.insert(spell);
            }
            GameAction::AdvanceTech(num_techs) => {
                self.tech_assignment.advance(num_techs);
            }
            GameAction::AcquireTech(tech_index) => {
                self.tech_assignment.acquire(tech_index);
            }
            GameAction::GiveSpell(board_index, spell) => {
                let k = self.spells.remove(&spell);
                ensure!(k > 0, "Cannot give non-drawn spell");
                self.spell_assignment[board_index] = spell;
            }
            GameAction::BoardAction(board_index, action) => {
                self.add_action(board_index, action);
            }        
        }

        Ok(())
    }

    /// Add an action to a specific board
    pub fn add_action(&mut self, board_index: usize, action: BoardAction) {
        if board_index < self.board_actions.len() {
            self.board_actions[board_index].push(action);
        }
    }
}

#[cfg(test)]
mod tests { }
