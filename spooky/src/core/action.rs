//! Game actions and moves

use super::{
    board::actions::{AttackAction, BoardTurn, SetupAction, SpawnAction},
    spells::{Spell, SpellCast},
    tech::TechAssignment,
};

use anyhow::{bail, ensure, Result};
use hashbag::HashBag;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameAction {
    BuySpell(Spell),
    AdvanceTech(usize),
    AcquireTech(usize),
    GiveSpell(usize, Spell),
    BoardSetupAction(usize, SetupAction),
    BoardAttackAction(usize, AttackAction),
    BoardSpawnAction(usize, SpawnAction),
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
            GameAction::AdvanceTech(num_techs) => {
                self.tech_assignment.advance(num_techs);
            }
            GameAction::AcquireTech(tech_index) => {
                self.tech_assignment.acquire(tech_index);
            }
            GameAction::GiveSpell(board_index, spell) => {
                let k = self.spells.remove(&spell);
                if k == 0 {
                    return Err(anyhow::anyhow!("Cannot give non-drawn spell"));
                }
                self.spell_assignment[board_index] = spell;
            }
            GameAction::BoardSetupAction(board_index, action) => {
                self.board_turns[board_index].setup_action = Some(action);
            }
            GameAction::BoardAttackAction(board_index, action) => {
                self.board_turns[board_index].attack_actions.push(action);
            }
            GameAction::BoardSpawnAction(board_index, action) => {
                self.board_turns[board_index].spawn_actions.push(action);
            }
        }

        Ok(())
    }
}

impl Display for GameTurn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();

        // Spells
        out.push_str("turn spells");
        for spell in self.spells.iter() {
            out.push_str(&format!(" {}", spell));
        }
        out.push('\n');

        // Tech
        if self.tech_assignment.advance_by > 0 {
            out.push_str(&format!("action adv_tech {}\n", self.tech_assignment.advance_by));
        }
        for &tech_idx in &self.tech_assignment.acquire {
            out.push_str(&format!("action acq_tech {}\n", tech_idx));
        }

        // Spell assignment
        for (i, &spell) in self.spell_assignment.iter().enumerate() {
            if spell != Spell::Blank {
                out.push_str(&format!("action give_spell {} {}\n", i, spell));
            }
        }

        // Board turns
        for (i, board_turn) in self.board_turns.iter().enumerate() {
            if let Some(action) = &board_turn.setup_action {
                out.push_str(&format!("action b_setup {} {}\n", i, action));
            }
            for action in &board_turn.attack_actions {
                out.push_str(&format!("action b_attack {} {}\n", i, action));
            }
            for action in &board_turn.spawn_actions {
                out.push_str(&format!("action b_spawn {} {}\n", i, action));
            }
        }

        out.push_str("endturn");
        write!(f, "{}", out.trim_end())
    }
}

impl GameAction {
    pub fn from_args(action_name: &str, args: &[&str]) -> Result<Self> {
        match action_name {
            "buy_spell" => {
                ensure!(args.len() == 1, "buy_spell requires 1 argument");
                let spell = args[0].parse()?;
                Ok(GameAction::BuySpell(spell))
            }
            "adv_tech" => {
                ensure!(args.len() == 1, "adv_tech requires 1 argument");
                let num_techs = args[0].parse()?;
                Ok(GameAction::AdvanceTech(num_techs))
            }
            "acq_tech" => {
                ensure!(args.len() == 1, "acq_tech requires 1 argument");
                let tech_index = args[0].parse()?;
                Ok(GameAction::AcquireTech(tech_index))
            }
            "give_spell" => {
                ensure!(args.len() == 2, "give_spell requires 2 arguments");
                let board_index = args[0].parse()?;
                let spell = args[1].parse()?;
                Ok(GameAction::GiveSpell(board_index, spell))
            }
            "b_setup" => {
                ensure!(args.len() >= 2, "board setup action requires at least 2 arguments");
                let board_index = args[0].parse()?;
                let action = SetupAction::from_args(args[1], &args[2..])?;
                Ok(GameAction::BoardSetupAction(board_index, action))
            }
            "b_attack" => {
                ensure!(args.len() >= 2, "board attack action requires at least 2 arguments");
                let board_index = args[0].parse()?;
                let action = AttackAction::from_args(args[1], &args[2..])?;
                Ok(GameAction::BoardAttackAction(board_index, action))
            }
            "b_spawn" => {
                ensure!(args.len() >= 2, "board spawn action requires at least 2 arguments");
                let board_index = args[0].parse()?;
                let action = SpawnAction::from_args(args[1], &args[2..])?;
                Ok(GameAction::BoardSpawnAction(board_index, action))
            }
            _ => bail!("Unknown game action: {}", action_name),
        }
    }
}

