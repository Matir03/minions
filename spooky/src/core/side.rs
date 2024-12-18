use anyhow::{anyhow, Result};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use super::convert::{FromIndex, ToIndex};

/// Side/player in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum Side {
    S0,
    S1,
}

impl FromIndex for Side {
    fn from_index(idx: usize) -> Result<Self> {
        FromPrimitive::from_usize(idx)
            .ok_or_else(|| anyhow!("Invalid side index: {}", idx))
    }
}

impl ToIndex for Side {
    fn to_index(&self) -> Result<usize> {
        ToPrimitive::to_usize(self)
            .ok_or_else(|| anyhow!("Invalid side value"))
    }
}

/// Array indexed by game side
#[derive(Debug, Clone)]
pub struct SideArray<T> {
    pub values: [T; 2],
}

impl<T> SideArray<T> {
    pub fn new(s0: T, s1: T) -> Self {
        Self {
            values: [s0, s1],
        }
    }

    pub fn get(&self, side: Side) -> Result<&T> {
        Ok(&self.values[side.to_index()?])
    }

    pub fn get_mut(&mut self, side: Side) -> Result<&mut T> {
        Ok(&mut self.values[side.to_index()?])
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.values.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.values.iter_mut()
    }
}
