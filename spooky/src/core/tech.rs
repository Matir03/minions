//! Tech system representations

use crate::core::tech;

use super::{
    convert::{FromIndex, ToIndex},
    side::{Side, SideArray},
    units::Unit,
};
use anyhow::{anyhow, bail, ensure, Result};
use std::{collections::HashSet, ops::Index};

pub const SPELL_COST: i32 = 4;

/// Status of a tech in the techline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TechStatus {
    Locked,
    Unlocked,
    Acquired,
}

/// Different tech types available in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tech {
    UnitTech(Unit),
    Copycat,
    Thaumaturgy,
    Metamagic,
}

impl Tech {
    pub fn counters(&self) -> Vec<Tech> {
        match self {
            Tech::UnitTech(unit) => {
                let unit_idx = unit.to_index().unwrap() as i32;
                [unit_idx + 1, unit_idx + 2, unit_idx - 3]
                    .into_iter()
                    .filter(|i| *i >= 1 && *i <= Unit::NUM_UNITS as i32)
                    .map(|i| Tech::UnitTech(Unit::from_index(i as usize).unwrap()))
                    .collect()
            }
            _ => vec![],
        }
    }
}

impl FromIndex for Tech {
    fn from_index(idx: usize) -> Result<Self> {
        Ok(match idx {
            1..=22 => Tech::UnitTech(Unit::from_index(idx + Unit::BASIC_UNITS.len() - 1)?),
            23 => Tech::Copycat,
            24 => Tech::Thaumaturgy,
            25 => Tech::Metamagic,
            _ => bail!("Invalid tech index: {}", idx),
        })
    }
}

impl ToIndex for Tech {
    fn to_index(&self) -> Result<usize> {
        Ok(match self {
            Tech::Copycat => 23,
            Tech::Thaumaturgy => 24,
            Tech::Metamagic => 25,
            Tech::UnitTech(unit) => unit.to_index()? - Unit::BASIC_UNITS.len() + 1,
        })
    }
}
pub const NUM_TECHS: usize = 25;

/// Represents the tech tree structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Techline {
    pub techs: Vec<Tech>,
}

impl Techline {
    /// Create a new techline with the specified number of techs
    pub fn new(techs: Vec<Tech>) -> Self {
        Self { techs }
    }

    pub fn len(&self) -> usize {
        self.techs.len()
    }

    pub fn index_of(&self, tech: Tech) -> usize {
        self.techs.iter().position(|&t| t == tech).unwrap()
    }
}

impl Index<usize> for Techline {
    type Output = Tech;

    fn index(&self, index: usize) -> &Self::Output {
        &self.techs[index]
    }
}

impl Default for Techline {
    fn default() -> Self {
        Self::new(
            (1..=NUM_TECHS)
                .map(Tech::from_index)
                .collect::<Result<Vec<_>>>()
                .unwrap(),
        )
    }
}

// assignment of techs during a generaling phase
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TechAssignment {
    pub advance_by: usize,
    pub acquire: Vec<usize>,
}

impl TechAssignment {
    pub fn new(advance_by: usize, acquire: Vec<usize>) -> Self {
        Self {
            advance_by,
            acquire,
        }
    }

    pub fn num_spells(&self) -> i32 {
        (self.advance_by + self.acquire.len()) as i32
    }

    pub fn advance(&mut self, advance_by: usize) {
        self.advance_by += advance_by;
    }

    pub fn acquire(&mut self, acquire: usize) {
        self.acquire.push(acquire);
    }
}

/// state of both teams' tech
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TechState {
    /// index of next tech to unlock
    pub unlock_index: SideArray<usize>,
    pub status: SideArray<[TechStatus; NUM_TECHS]>,
    pub acquired_techs: SideArray<HashSet<Tech>>,
}

impl TechState {
    pub fn new() -> Self {
        Self {
            unlock_index: SideArray::new(0, 0),
            status: SideArray::new(
                [TechStatus::Locked; NUM_TECHS],
                [TechStatus::Locked; NUM_TECHS],
            ),
            acquired_techs: SideArray::new(HashSet::new(), HashSet::new()),
        }
    }

    pub fn acquirable(
        &self,
        tech: Tech,
        techline: &Techline,
        side: Side,
        num_buyable_spells: i32,
    ) -> bool {
        let maybe_tech_index = techline.techs.iter().position(|&t| t == tech);

        let tech_index = match maybe_tech_index {
            Some(i) => i,
            None => return false,
        };

        if self.status[side][tech_index] == TechStatus::Acquired
            || self.status[!side][tech_index] == TechStatus::Acquired
        {
            return false;
        }

        let spells_to_buy = (tech_index as i32 - self.unlock_index[side] as i32 + 1).max(0);
        spells_to_buy <= num_buyable_spells
    }

    pub fn assign_techs(
        &mut self,
        assignment: TechAssignment,
        side: Side,
        techline: &Techline,
    ) -> Result<()> {
        let advanced_index = self.unlock_index[side] + assignment.advance_by;
        ensure!(
            advanced_index <= techline.techs.len(),
            "Cannot advance past last tech"
        );

        for i in self.unlock_index[side]..advanced_index {
            self.status[side][i] = TechStatus::Unlocked;
        }

        self.unlock_index[side] = advanced_index;

        for tech_index in assignment.acquire {
            let tech = techline[tech_index];
            ensure!(
                tech_index < self.unlock_index[side],
                "Cannot acquire locked tech"
            );
            ensure!(
                tech == Tech::Copycat
                    || self.acquired_techs[side].contains(&Tech::Copycat)
                    || self.status[!side][tech_index] != TechStatus::Acquired,
            );
            self.status[side][tech_index] = TechStatus::Acquired;
            self.acquired_techs[side].insert(techline[tech_index]);
        }

        Ok(())
    }

    pub fn can_buy(&self, side: Side, unit: Unit) -> bool {
        self.acquired_techs[side].contains(&Tech::UnitTech(unit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tech_conversion() {
        // Test special techs
        assert_eq!(Tech::from_index(23).unwrap(), Tech::Copycat);
        assert_eq!(Tech::from_index(24).unwrap(), Tech::Thaumaturgy);
        assert_eq!(Tech::from_index(25).unwrap(), Tech::Metamagic);

        assert_eq!(Tech::Copycat.to_index().unwrap(), 23);
        assert_eq!(Tech::Thaumaturgy.to_index().unwrap(), 24);
        assert_eq!(Tech::Metamagic.to_index().unwrap(), 25);

        // Test invalid index
        assert!(Tech::from_index(27).is_err());

        // Test unit tech - with BASIC_UNITS = [Zombie], first tech is Initiate at index 1
        assert_eq!(Tech::from_index(1).unwrap(), Tech::UnitTech(Unit::Initiate));
        assert_eq!(Tech::UnitTech(Unit::Initiate).to_index().unwrap(), 1);
        assert_eq!(Tech::from_index(2).unwrap(), Tech::UnitTech(Unit::Skeleton));
        assert_eq!(Tech::UnitTech(Unit::Skeleton).to_index().unwrap(), 2);
    }

    #[test]
    fn test_techline() {
        let techs = vec![
            Tech::UnitTech(Unit::from_index(4).unwrap()),
            Tech::UnitTech(Unit::from_index(2).unwrap()),
            Tech::Thaumaturgy,
        ];

        let techline = Techline::new(techs);
        assert_eq!(techline.techs.len(), 3);
    }
}
