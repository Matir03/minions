//! Tech system representations

use super::units::UnitLabel;
use anyhow::{anyhow, Result};
use super::convert::{FromIndex, ToIndex};

/// Status of a tech in the tech tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TechStatus {
    Locked,
    Unlocked,
    Acquired,
}

/// Different tech types available in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tech {
    UnitTech(UnitLabel),
    Copycat,
    Thaumaturgy,
    Metamagic,
}

impl FromIndex for Tech {
    fn from_index(idx: usize) -> Result<Self> {
        Ok(match idx {
            24 => Tech::Copycat,
            25 => Tech::Thaumaturgy,
            26 => Tech::Metamagic,
            _ => Tech::UnitTech(UnitLabel::from_index(idx)?),
        })
    }
}

impl ToIndex for Tech {
    fn to_index(&self) -> Result<usize> {
        Ok(match self {
            Tech::Copycat => 24,
            Tech::Thaumaturgy => 25,
            Tech::Metamagic => 26,
            Tech::UnitTech(unit) => unit.to_index()?,
        })
    }
}

/// Represents the tech tree structure
#[derive(Debug, Clone)]
pub struct Techline {
    pub num_techs: usize,
    pub techs: Vec<Tech>,
}

impl Techline {
    /// Create a new tech tree with the specified number of techs
    pub fn new(num_techs: usize) -> Self {
        Self {
            num_techs,
            techs: Vec::with_capacity(num_techs),
        }
    }
}
