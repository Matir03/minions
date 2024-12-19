//! Tech system representations

use super::units::UnitLabel;
use anyhow::{bail, Result};
use super::convert::{FromIndex, ToIndex};

/// Status of a tech in the techline
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
            1..=23 => Tech::UnitTech(UnitLabel::from_index(idx)?),
            24 => Tech::Copycat,
            25 => Tech::Thaumaturgy,
            26 => Tech::Metamagic,
            _ => bail!("Invalid tech index: {}", idx),
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
    pub techs: Vec<Tech>,
}

impl Techline {
    pub const NUM_TECHS: usize = 26;
    /// Create a new techline with the specified number of techs
    pub fn new(techs: Vec<Tech>) -> Self {
        Self { techs }
    }
}

impl Default for Techline {
    fn default() -> Self {
        Self::new(
            (1..=Techline::NUM_TECHS)
            .map(Tech::from_index)
            .collect::<Result<Vec<_>>>()
            .unwrap()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tech_conversion() {
        // Test special techs
        assert_eq!(Tech::from_index(24).unwrap(), Tech::Copycat);
        assert_eq!(Tech::from_index(25).unwrap(), Tech::Thaumaturgy);
        assert_eq!(Tech::from_index(26).unwrap(), Tech::Metamagic);

        assert_eq!(Tech::Copycat.to_index().unwrap(), 24);
        assert_eq!(Tech::Thaumaturgy.to_index().unwrap(), 25);
        assert_eq!(Tech::Metamagic.to_index().unwrap(), 26);

        // Test invalid index
        assert!(Tech::from_index(27).is_err());

        // Test unit tech
        assert_eq!(Tech::from_index(1).unwrap(), Tech::UnitTech(UnitLabel::Initiate));
        assert_eq!(Tech::UnitTech(UnitLabel::Initiate).to_index().unwrap(), 1);
    }

    #[test]
    fn test_techline() {
        let techs = vec![
            Tech::UnitTech(UnitLabel::from_index(4).unwrap()),
            Tech::UnitTech(UnitLabel::from_index(2).unwrap()),
            Tech::Thaumaturgy,
        ];

        let techline = Techline::new(techs);
        assert_eq!(techline.techs.len(), 3);
    }
}
