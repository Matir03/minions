//! Tech system representations

use crate::core::tech;

use super::{
    units::Unit, 
    side::{Side, SideArray},
    convert::{FromIndex, ToIndex}
};
use anyhow::{anyhow, bail, Result, ensure};
use std::{
    ops::Index,
    collections::HashSet as Set
};

pub const SPELL_COST: i32 = 4;

/// Status of a tech in the techline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TechStatus {
    Locked,
    Unlocked,
    Acquired,
}

/// Different tech types available in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tech {
    UnitTech(Unit),
    Copycat,
    Thaumaturgy,
    Metamagic,
}

impl FromIndex for Tech {
    fn from_index(idx: usize) -> Result<Self> {
        Ok(match idx {
            1..=23 => Tech::UnitTech(Unit::from_index(idx)?),
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
pub const NUM_TECHS: usize = 26;

/// Represents the tech tree structure
#[derive(Debug, Clone)]
pub struct Techline {
    pub techs: Vec<Tech>,
}

impl Techline {
    /// Create a new techline with the specified number of techs
    pub fn new(techs: Vec<Tech>) -> Self {
        Self { techs }
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
            .unwrap()
        )
    }
}

// assignment of techs during a generaling phase
#[derive(Debug, Clone, Default)]
pub struct TechAssignment {
    pub advance_by: usize,
    pub acquire: Vec<Tech>,
}

impl TechAssignment {
    pub fn new(advance_by: usize, acquire: Vec<Tech>) -> Self {
        Self { advance_by, acquire }
    }

    pub fn num_spells(&self) -> usize {
        self.advance_by + self.acquire.len()
    }
}

// state of both teams' tech
#[derive(Debug, Clone)]
pub struct TechState {
    pub unlock_index: SideArray<usize>,
    pub status: SideArray<[TechStatus; NUM_TECHS]>,
    pub acquired_techs: SideArray<Set<Tech>>
}

impl TechState {
    pub fn new() -> Self {
        Self {
            unlock_index: SideArray::new(0, 0),
            status: SideArray::new(
                [TechStatus::Locked; NUM_TECHS],
                [TechStatus::Locked; NUM_TECHS],
            ),
            acquired_techs: SideArray::new(
                Set::new(),
                Set::new(),
            )
        }
    }

    pub fn assign_techs(&mut self, assignment: TechAssignment, side: Side, techline: &Techline) -> Result<()> {
        let advanced_index = self.unlock_index[side] + assignment.advance_by;
        ensure!(advanced_index < techline.techs.len(), "Cannot advance past last tech");

        for i in (self.unlock_index[side] + 1)..=advanced_index {
            self.status[side][i] = TechStatus::Unlocked;
        }

        self.unlock_index[side] = advanced_index;

        for tech in assignment.acquire {
            let tech_index = tech.to_index()?;
            ensure!(tech_index <= self.unlock_index[side], "Cannot acquire locked tech");
            ensure!(
                tech == Tech::Copycat ||
                self.acquired_techs[side].contains(&Tech::Copycat) ||
                self.status[!side][tech_index] != TechStatus::Acquired,
            );
            self.status[side][tech_index] = TechStatus::Acquired;
            self.acquired_techs[side].insert(techline[tech_index]);
        }

        Ok(())
    }

    pub fn can_buy(&self, side: Side, unit: Unit) -> bool {
        self.acquired_techs[side].contains(&Tech::UnitTech(unit))
    }

    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        for (i, side_techs) in self.status.iter().enumerate() {
            if i > 0 {
                fen.push('|');
            }
            for status in side_techs {
                fen.push(match status {
                    TechStatus::Locked => 'L',
                    TechStatus::Unlocked => 'U',
                    TechStatus::Acquired => 'A',
                });
            }
        }

        fen
    }

    pub fn from_fen(fen: &str, techline: &Techline) -> Result<Self> {
        let mut state = Self::new();
        let num_techs = techline.techs.len();

        let side_strs = fen.split('|');

        ensure!(side_strs.clone().count() == 2);
        for (side_index, side_str) in side_strs.enumerate() {
            ensure!(side_str.len() == num_techs);

            let side = Side::from_index(side_index)?;
            for (i, c) in side_str.chars().enumerate() {
                state.status[side][i] = match c {
                    'L' => { 
                        state.unlock_index[side] = i;
                        break; 
                    }
                    'U' => TechStatus::Unlocked,
                    'A' => {
                        state.acquired_techs[side].insert(techline[i]);
                        TechStatus::Acquired
                    }
                    _ => bail!("Invalid tech status"),
                };
            }
        }

        state.unlock_index.values = state.status.values
            .map(|side_techs| 
                side_techs
                    .iter()
                    .position(|&status| status == TechStatus::Locked)
                    .unwrap_or(1) - 1
            );
        
        state.acquired_techs.values = state.status.values
            .map(|side_techs| 
                side_techs
                    .iter()
                    .enumerate()
                    .filter(|(_, &status)| status == TechStatus::Acquired)
                    .map(|(i, _)| techline[i])
                    .collect()
            );

        Ok(state)
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
        assert_eq!(Tech::from_index(1).unwrap(), Tech::UnitTech(Unit::Initiate));
        assert_eq!(Tech::UnitTech(Unit::Initiate).to_index().unwrap(), 1);
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
