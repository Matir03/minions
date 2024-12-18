//! Map and hex grid representations

use anyhow::{anyhow, Result};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use super::convert::{FromIndex, ToIndex};

/// Type of tile in the hex grid
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    Land,
    Water,
    Graveyard,
    // Add other tile types
}

/// Represents a location on the hex grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Loc {
    pub row: i32,
    pub col: i32,
}

/// Grid of hexagonal tiles
#[derive(Debug, Clone)]
pub struct HexArray<T> {
    width: usize,
    height: usize,
    tiles: Vec<T>,
}

impl<T: Clone> HexArray<T> {
    /// Create a new hex grid with given dimensions
    pub fn new(width: usize, height: usize, default: T) -> Self {
        Self {
            width,
            height,
            tiles: vec![default; width * height],
        }
    }

    /// Get tile at specified location
    pub fn get(&self, loc: Loc) -> Option<&T> {
        if self.in_bounds(loc) {
            Some(&self.tiles[self.index(loc)])
        } else {
            None
        }
    }

    /// Set tile at specified location
    pub fn set(&mut self, loc: Loc, value: T) -> bool {
        if self.in_bounds(loc) {
            let index = self.index(loc);
            self.tiles[index] = value;
            true
        } else {
            false
        }
    }

    fn in_bounds(&self, loc: Loc) -> bool {
        loc.row >= 0 && loc.row < self.width as i32 && 
        loc.col >= 0 && loc.col < self.height as i32
    }

    fn index(&self, loc: Loc) -> usize {
        (loc.col as usize) * self.width + (loc.row as usize)
    }
}

/// Different map types available in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum MapLabel {
    BlackenedShores = 0,
    MidnightLake = 1,
}

impl FromIndex for MapLabel {
    fn from_index(idx: usize) -> Result<Self> {
        FromPrimitive::from_usize(idx)
            .ok_or_else(|| anyhow!("Invalid map index: {}", idx))
    }
}

impl ToIndex for MapLabel {
    fn to_index(&self) -> Result<usize> {
        ToPrimitive::to_usize(self)
            .ok_or_else(|| anyhow!("Invalid map label"))
    }
}

/// Represents a game map with its hex grid
#[derive(Debug, Clone)]
pub struct Map {
    pub hexes: HexArray<TileType>,
    pub label: MapLabel,
}
