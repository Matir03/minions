//! Map and hex grid representations

use super::{
    convert::{FromIndex, ToIndex},
    loc::{Loc, GRID_LEN, GRID_SIZE},
    units::Unit,
};
use anyhow::{anyhow, bail, ensure, Context, Result};
use indoc::indoc;
use lazy_static::lazy_static;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

/// Type of tile in the hex grid
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    Ground,
    Graveyard,
    // Add other tile types
    NativeTerrain(Terrain),
}

impl TileType {
    pub fn to_display_char(&self) -> char {
        match self {
            TileType::Ground => '.',
            TileType::Graveyard => '$',
            TileType::NativeTerrain(t) => '.',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn from_fen_char(c: char) -> Option<Terrain> {
        match c {
            'A' => Some(Terrain::Whirlwind),
            'W' => Some(Terrain::Flood),
            'E' => Some(Terrain::Earthquake),
            'F' => Some(Terrain::Firestorm),
            _ => None,
        }
    }

    pub fn to_display_char(&self) -> char {
        match self {
            Terrain::Flood => '~',
            Terrain::Earthquake => 'E',
            Terrain::Whirlwind => 'W',
            Terrain::Firestorm => 'F',
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
    pub graveyards: Vec<Loc>,
}

impl MapSpec {
    pub fn new() -> Self {
        Self {
            tiles: HexGrid::new([TileType::Ground; GRID_SIZE]),
            graveyards: vec![],
        }
    }

    pub fn from_fen(fen: &str) -> Result<Self> {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        let mut spec = MapSpec::new();

        for c in fen.chars() {
            match c {
                '/' => {
                    ensure!(x == 10, "Invalid FEN: wrong number of squares in x");
                    ensure!(y < 10, "Invalid FEN: too many rows");
                    y += 1;
                    x = 0;
                }
                '0'..='9' => {
                    let empty = if c == '0' {
                        10
                    } else {
                        c.to_digit(10).unwrap() as i32
                    };
                    x += empty;
                }
                'G' => {
                    ensure!(
                        x < 10 && y < 10,
                        "Invalid FEN: graveyard position out of bounds"
                    );
                    let loc = Loc { x, y };

                    spec.graveyards.push(loc);
                    spec.tiles.set(&loc, TileType::Graveyard);
                    x += 1;
                }
                'A'..='Z' => {
                    ensure!(
                        x < 10 && y < 10,
                        "Invalid FEN: terrain position out of bounds"
                    );

                    let terrain =
                        Terrain::from_fen_char(c).context("Invalid FEN: unknown terrain")?;

                    let loc = Loc { x, y };

                    spec.tiles.set(&loc, TileType::NativeTerrain(terrain));
                    x += 1;
                }
                _ => bail!("Invalid FEN: unexpected character {}", c),
            }
        }

        Ok(spec)
    }
}

/// Grid of hexagonal tiles
#[derive(Debug, Clone)]
pub struct HexGrid<T> {
    tiles: [T; GRID_SIZE],
}

impl<T> HexGrid<T> {
    /// Create a new hex grid with given dimensions
    pub const fn new(tiles: [T; GRID_SIZE]) -> Self {
        Self { tiles }
    }

    /// Get tile at specified location
    pub fn get(&self, loc: &Loc) -> Option<&T> {
        if self.in_bounds(loc) {
            Some(&self.tiles[self.index(loc)])
        } else {
            None
        }
    }

    /// Set tile at specified location
    pub fn set(&mut self, loc: &Loc, value: T) -> bool {
        if self.in_bounds(loc) {
            let index = self.index(loc);
            self.tiles[index] = value;
            true
        } else {
            false
        }
    }

    fn in_bounds(&self, loc: &Loc) -> bool {
        loc.y >= 0 && loc.y < GRID_LEN as i32 && loc.x >= 0 && loc.x < GRID_LEN as i32
    }

    fn index(&self, loc: &Loc) -> usize {
        (loc.x as usize) * GRID_LEN + (loc.y as usize)
    }
}

/// Different map types available in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum Map {
    AllLand,
    BlackenedShores,
    MidnightLake,
}

const NUM_MAPS: usize = 3;

impl Map {
    pub fn spec(&self) -> &MapSpec {
        &MAPS[*self as usize]
    }

    pub fn get_terrain(&self, loc: &Loc) -> Option<Terrain> {
        match self.spec().tiles.get(loc) {
            Some(TileType::NativeTerrain(t)) => Some(*t),
            _ => None,
        }
    }
}

impl FromIndex for Map {
    fn from_index(idx: usize) -> Result<Self> {
        FromPrimitive::from_usize(idx).ok_or_else(|| anyhow!("Invalid map index: {}", idx))
    }
}

impl ToIndex for Map {
    fn to_index(&self) -> Result<usize> {
        ToPrimitive::to_usize(self).ok_or_else(|| anyhow!("Invalid map label"))
    }
}

impl Default for Map {
    fn default() -> Self {
        Map::AllLand
    }
}

const MAP_FENS: [&str; NUM_MAPS] = [
    // AllLand
    "0/0/0/0/0/0/0/0/0/0",
    // BlackenedShores
    "2G3G3/W9/W9/W4G3G/GW4G3/W2WW5/WG2W5/W1W6G/3G1W4/2WWWGWWW1",
    // MidnightLake
    "1G1WW3G1/9G/0/G2G1WW3/W3WWW3/4WW3W/1G4G2W/0/3G5G/5WG3",
];

lazy_static! {
    pub static ref MAPS: [MapSpec; NUM_MAPS] = MAP_FENS
        .iter()
        .map(|fen| MapSpec::from_fen(fen).unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_label_conversion() {
        assert_eq!(Map::from_index(0).unwrap(), Map::AllLand);
        assert_eq!(Map::from_index(1).unwrap(), Map::BlackenedShores);
        assert_eq!(Map::from_index(2).unwrap(), Map::MidnightLake);
        assert!(Map::from_index(3).is_err());

        assert_eq!(Map::AllLand.to_index().unwrap(), 0);
        assert_eq!(Map::BlackenedShores.to_index().unwrap(), 1);
        assert_eq!(Map::MidnightLake.to_index().unwrap(), 2);
    }

    #[test]
    fn test_hex_array() {
        let mut array = HexGrid::new([TileType::Ground; GRID_SIZE]);

        // Test in_bounds
        assert!(array.in_bounds(&Loc { y: 0, x: 0 }));
        assert!(array.in_bounds(&Loc { y: 9, x: 9 }));
        assert!(!array.in_bounds(&Loc { y: 10, x: 0 }));
        assert!(!array.in_bounds(&Loc { y: 0, x: 10 }));
        assert!(!array.in_bounds(&Loc { y: -1, x: 0 }));

        // Test get/set
        let loc = Loc { y: 5, x: 5 };
        assert_eq!(array.get(&loc), Some(&TileType::Ground));
        array.set(&loc, TileType::Graveyard);
        assert_eq!(array.get(&loc), Some(&TileType::Graveyard));
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
