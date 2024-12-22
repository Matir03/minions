//! Map and hex grid representations

use anyhow::{anyhow, Result};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use super::{
    convert::{FromIndex, ToIndex},
    loc::{Loc, GRID_LEN, GRID_SIZE},
    units::Unit,
};

/// Type of tile in the hex grid
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    Ground,
    Graveyard,
    // Add other tile types
}

pub enum Terrain {
    Flood,
    Earthquake,
    Whirlwind,
    Firestorm,
}

impl Terrain {
    pub fn allows(&self, unit: &Unit) -> bool {
        let stats = unit.stats();

        match self {
            Terrain::Flood => stats.flying,
            Terrain::Earthquake => stats.speed >= 2,
            Terrain::Whirlwind => stats.persistent,
            Terrain::Firestorm => stats.defense >= 4,
        }
    }
}

impl Default for TileType {
    fn default() -> Self {
        TileType::Ground
    }
}

/// Represents a game map with its hex grid
#[derive(Debug, Clone)]
pub struct MapSpec {
    pub tiles: HexGrid<TileType>,
}

/// Grid of hexagonal tiles
#[derive(Debug, Clone)]
pub struct HexGrid<T> {
    tiles: [T; GRID_SIZE],
}

impl<T> HexGrid<T> {
    /// Create a new hex grid with given dimensions
    pub const fn new(tiles: [T; GRID_SIZE]) -> Self {
        Self { tiles, }
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
        loc.y >= 0 && loc.y < GRID_LEN as i32 && 
        loc.x >= 0 && loc.x < GRID_LEN as i32
    }

    fn index(&self, loc: Loc) -> usize {
        (loc.x as usize) * GRID_LEN + (loc.y as usize)
    }
}

/// Different map types available in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum Map {
    BlackenedShores = 0,
    MidnightLake = 1,
}

const NUM_MAPS: usize = 2;

impl Map {
    pub fn spec(&self) -> &MapSpec {
        &MAPS[*self as usize]
    }
}

impl FromIndex for Map {
    fn from_index(idx: usize) -> Result<Self> {
        FromPrimitive::from_usize(idx)
            .ok_or_else(|| anyhow!("Invalid map index: {}", idx))
    }
}

impl ToIndex for Map {
    fn to_index(&self) -> Result<usize> {
        ToPrimitive::to_usize(self)
            .ok_or_else(|| anyhow!("Invalid map label"))
    }
}

const MAPS: [MapSpec; NUM_MAPS] = [
    // TODO: Add map specs
    MapSpec {
        tiles: HexGrid::new([TileType::Ground; GRID_SIZE]),
    },

    MapSpec {
        tiles: HexGrid::new([TileType::Ground; GRID_SIZE]),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_label_conversion() {
        assert_eq!(Map::from_index(0).unwrap(), Map::BlackenedShores);
        assert_eq!(Map::from_index(1).unwrap(), Map::MidnightLake);
        assert!(Map::from_index(2).is_err());

        assert_eq!(Map::BlackenedShores.to_index().unwrap(), 0);
        assert_eq!(Map::MidnightLake.to_index().unwrap(), 1);
    }

    #[test]
    fn test_hex_array() {
        let mut array = HexGrid::new([TileType::Ground; GRID_SIZE]);
        
        // Test in_bounds
        assert!(array.in_bounds(Loc { y: 0, x: 0 }));
        assert!(array.in_bounds(Loc { y: 9, x: 9 }));
        assert!(!array.in_bounds(Loc { y: 10, x: 0 }));
        assert!(!array.in_bounds(Loc { y: 0, x: 10 }));
        assert!(!array.in_bounds(Loc { y: -1, x: 0 }));
        
        // Test get/set
        let loc = Loc { y: 5, x: 5 };
        assert_eq!(array.get(loc), Some(&TileType::Ground));
        array.set(loc, TileType::Graveyard);
        assert_eq!(array.get(loc), Some(&TileType::Graveyard));
    }

    // #[test]
    // fn test_map() {
    //     let map = MapSpec {
    //         tiles: HexGrid::new([TileType::Land; GRID_SIZE]),
    //     };
    // }

    #[test]
    fn test_loc() {
        let loc = Loc { y: 3, x: 4 };
        assert_eq!(loc.y, 3);
        assert_eq!(loc.x, 4);
    }
}
