use super::{
    side::Side,
    board::Board,
    units::Unit,
    loc::Loc,
};
use anyhow::anyhow;
use std::str::FromStr;

/// Types of spells that can be cast
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Spell {
    Blank,
    Unknown,
    Shield,
    Reposition,
    // TODO: Add other spell types
}

impl FromStr for Spell {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "blank" => Ok(Spell::Blank),
            "unknown" => Ok(Spell::Unknown),
            "shield" => Ok(Spell::Shield),
            "reposition" => Ok(Spell::Reposition),
            _ => Err(anyhow!("Unknown spell: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellCast {
    CastShield {
        target: Loc,
    },
    CastReposition {
        from_sq: Loc,
        to_sq: Loc,
    }
}

use anyhow::Result;

impl SpellCast {
    pub fn cast(&self, board: &mut Board, caster: Side) -> Result<()> {
        todo!();
    }
}

