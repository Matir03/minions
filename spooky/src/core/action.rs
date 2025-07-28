//! Game actions and moves

use super::{
    board::actions::{AttackAction, BoardTurn, SetupAction, SpawnAction},
    spells::{Spell, SpellCast},
    tech::TechAssignment,
};

use anyhow::{bail, ensure, Context, Result};
use hashbag::HashBag;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardAction {
    Setup(SetupAction),
    Attack(AttackAction),
    Spawn(SpawnAction),
}

impl BoardAction {
    pub fn from_args(action_name: &str, args: &[&str]) -> Result<Self> {
        let setup_action = SetupAction::from_args(action_name, args);
        if setup_action.is_ok() {
            return Ok(BoardAction::Setup(setup_action?));
        }
        let attack_action = AttackAction::from_args(action_name, args);
        if attack_action.is_ok() {
            return Ok(BoardAction::Attack(attack_action?));
        }
        let spawn_action = SpawnAction::from_args(action_name, args);
        if spawn_action.is_ok() {
            return Ok(BoardAction::Spawn(spawn_action?));
        }

        bail!("Unknown board action: {}", action_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameAction {
    BuySpell(Spell),
    GiveSpell(usize, Spell),
    TechAction(TechAssignment),
    BoardAction(usize, BoardAction),
}

/// Represents a complete turn in the game
#[derive(Debug, Clone)]
pub struct GameTurn {
    pub spells: HashBag<Spell>,
    pub spell_assignment: Vec<Spell>, // board -> spell index
    pub tech_assignment: TechAssignment,
    pub board_turns: Vec<BoardTurn>,
}

impl GameTurn {
    /// Create a new turn with the given number of boards
    pub fn new(spells: Vec<Spell>, num_boards: usize) -> Self {
        assert!(spells.len() == num_boards + 1, "Invalid number of spells");

        Self {
            spells: spells.into_iter().collect(),
            spell_assignment: vec![Spell::Blank; num_boards],
            tech_assignment: TechAssignment::default(),
            board_turns: vec![BoardTurn::default(); num_boards],
        }
    }

    pub fn do_action(&mut self, action: GameAction) -> Result<()> {
        match action {
            GameAction::BuySpell(spell) => {
                self.spells.insert(spell);
            }
            GameAction::TechAction(tech_assignment) => {
                self.tech_assignment = tech_assignment;
            }
            GameAction::GiveSpell(board_index, spell) => {
                let k = self.spells.remove(&spell);
                if k == 0 {
                    return Err(anyhow::anyhow!("Cannot give non-drawn spell"));
                }
                self.spell_assignment[board_index] = spell;
            }
            GameAction::BoardAction(board_index, action) => match action {
                BoardAction::Setup(action) => {
                    self.board_turns[board_index].setup_action = Some(action);
                }
                BoardAction::Attack(action) => {
                    self.board_turns[board_index].attack_actions.push(action);
                }
                BoardAction::Spawn(action) => {
                    self.board_turns[board_index].spawn_actions.push(action);
                }
            },
        }

        Ok(())
    }
}

impl Display for GameTurn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();

        // Spells
        out.push_str("turn");
        if !self.spells.is_empty() {
            out.push_str(" spells");
            for spell in self.spells.iter() {
                out.push_str(&format!(" {}", spell));
            }
        }
        out.push('\n');

        // Tech
        out.push_str(&format!("action tech {}", self.tech_assignment.advance_by));
        for &tech_idx in &self.tech_assignment.acquire {
            out.push_str(&format!(" {}", tech_idx));
        }
        out.push('\n');

        // Spell assignment
        for (i, &spell) in self.spell_assignment.iter().enumerate() {
            if spell != Spell::Blank {
                out.push_str(&format!("action give_spell {} {}\n", i, spell));
            }
        }

        // Board turns
        for (i, board_turn) in self.board_turns.iter().enumerate() {
            if let Some(action) = &board_turn.setup_action {
                out.push_str(&format!("action board {} {}\n", i, action));
            }
            for action in &board_turn.attack_actions {
                out.push_str(&format!("action board {} {}\n", i, action));
            }
            for action in &board_turn.spawn_actions {
                out.push_str(&format!("action board {} {}\n", i, action));
            }
        }

        out.push_str("endturn");
        write!(f, "{}", out.trim_end())
    }
}

impl GameAction {
    pub fn try_from_args(action_name: &str, args: &[&str]) -> Result<Self> {
        match action_name {
            "buy_spell" => {
                ensure!(args.len() == 1, "buy_spell requires 1 argument");
                let spell = args[0].parse()?;
                Ok(GameAction::BuySpell(spell))
            }
            "tech" => {
                ensure!(args.len() >= 1, "tech requires at least 1 argument");
                let advance_by = args[0].parse()?;
                let acquire = args[1..]
                    .iter()
                    .map(|s| s.parse().context("Invalid tech index"))
                    .collect::<Result<Vec<_>>>()?;

                Ok(GameAction::TechAction(TechAssignment::new(
                    advance_by, acquire,
                )))
            }
            "give_spell" => {
                ensure!(args.len() == 2, "give_spell requires 2 arguments");
                let board_index = args[0].parse()?;
                let spell = args[1].parse()?;
                Ok(GameAction::GiveSpell(board_index, spell))
            }
            "board" => {
                ensure!(
                    args.len() >= 3,
                    "board action requires at least 3 arguments"
                );
                let board_index = args[0].parse()?;
                let action_name = args[1];
                let action_args = &args[2..];
                let action = BoardAction::from_args(action_name, action_args)?;
                Ok(GameAction::BoardAction(board_index, action))
            }
            _ => bail!("Unknown game action: {}", action_name),
        }
    }
}
