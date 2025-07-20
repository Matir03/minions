use super::{board::Board, loc::Loc, side::Side, units::Unit};
use anyhow::{anyhow, ensure};
use std::{fmt::Display, str::FromStr};

/// Types of spells that can be cast
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Spell {
    Blank,
    Unknown,
    Shield,
    Reposition,
    // TODO: Add other spell types
}

impl Display for Spell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Spell::Blank => write!(f, "blank"),
            Spell::Unknown => write!(f, "unknown"),
            Spell::Shield => write!(f, "shield"),
            Spell::Reposition => write!(f, "reposition"),
        }
    }
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
    CastShield { target: Loc },
    CastReposition { from_sq: Loc, to_sq: Loc },
}

use anyhow::Result;

impl SpellCast {
    pub fn cast(&self, board: &mut Board, caster: Side) -> Result<()> {
        todo!();
    }
}

impl Display for SpellCast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpellCast::CastShield { target } => write!(f, "shield {}", target),
            SpellCast::CastReposition { from_sq, to_sq } => {
                write!(f, "reposition {} {}", from_sq, to_sq)
            }
        }
    }
}

impl FromStr for SpellCast {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        ensure!(!parts.is_empty(), "Empty spell cast string");

        match parts[0] {
            "shield" => {
                ensure!(parts.len() == 2, "shield cast requires 1 argument");
                let target = parts[1].parse()?;
                Ok(SpellCast::CastShield { target })
            }
            "reposition" => {
                ensure!(parts.len() == 3, "reposition cast requires 2 arguments");
                let from_sq = parts[1].parse()?;
                let to_sq = parts[2].parse()?;
                Ok(SpellCast::CastReposition { from_sq, to_sq })
            }
            _ => Err(anyhow!("Unknown spell cast: {}", parts[0])),
        }
    }
}
