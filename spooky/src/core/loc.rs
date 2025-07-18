use anyhow::Context;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Add, Neg, Sub},
    str::FromStr,
};
use z3;

pub const GRID_LEN: usize = 10;
pub const GRID_SIZE: usize = GRID_LEN * GRID_LEN;

/// A location on the game board
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Loc {
    pub x: i32,
    pub y: i32,
}

impl Loc {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn in_bounds(&self) -> bool {
        self.x >= 0 && self.x < GRID_LEN as i32 && self.y >= 0 && self.y < GRID_LEN as i32
    }

    pub fn from_index(index: i32) -> Self {
        Self {
            x: index % GRID_LEN as i32,
            y: index / GRID_LEN as i32,
        }
    }

    pub fn neighbors(&self) -> Vec<Loc> {
        DIRS.into_iter()
            .map(|dir| self + &dir.into())
            .filter(|loc| loc.in_bounds())
            .collect()
    }

    pub fn dist(&self, other: &Loc) -> i32 {
        (self - other).length()
    }

    pub fn unsigned_dist(&self, other: &Loc) -> u32 {
        self.dist(other) as u32
    }

    pub fn index(&self) -> usize {
        (self.y as usize) * GRID_LEN + (self.x as usize)
    }

    pub fn paths_to(&self, other: &Loc) -> Vec<Vec<Loc>> {
        let delta = other - self;
        let dist = self.dist(other);
        let paths = PATH_MAPS[dist as usize].get(&delta).unwrap();
        paths
            .iter()
            .map(|path| path.iter().map(|delta| self + delta).collect::<Vec<Loc>>())
            .collect()
    }

    // Convert Loc to Z3 bitvector representation
    pub fn as_z3<'ctx>(self, ctx: &'ctx z3::Context) -> z3::ast::BV<'ctx> {
        use z3::ast::BV;
        if self.in_bounds() {
            let val = (self.x as u8) << 4 | (self.y as u8);
            BV::from_u64(ctx, val as u64, 8)
        } else {
            BV::from_u64(ctx, u8::MAX as u64, 8)
        }
    }

    // Convert from Z3 bitvector representation back to Loc
    pub fn from_z3(val: u64) -> Self {
        if val == u8::MAX as u64 {
            // Out of bounds marker
            Self { x: -1, y: -1 }
        } else {
            let x = ((val >> 4) & 0xF) as i32;
            let y = (val & 0xF) as i32;
            Self { x, y }
        }
    }

    pub fn parse_loc_pair(s: &str) -> anyhow::Result<(Loc, Loc)> {
        anyhow::ensure!(s.len() == 4, "Invalid location pair string");
        let from_str = &s[0..2];
        let to_str = &s[2..4];
        let from_loc = from_str.parse()?;
        let to_loc = to_str.parse()?;
        Ok((from_loc, to_loc))
    }
}

impl From<(i32, i32)> for Loc {
    fn from((x, y): (i32, i32)) -> Self {
        Self { x, y }
    }
}

impl FromStr for Loc {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        anyhow::ensure!(s.len() == 2, "Invalid location string");
        let x = s.chars().next().unwrap() as i32 - 'a' as i32;
        let y = s[1..].parse()?;
        let loc = Loc { x, y };
        anyhow::ensure!(loc.in_bounds(), "Location out of bounds");
        Ok(loc)
    }
}

impl Display for Loc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", (self.x as u8 + b'a') as char, self.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocDelta {
    pub dx: i32,
    pub dy: i32,
}

impl LocDelta {
    pub fn neighbors(&self) -> [LocDelta; 6] {
        DIRS.map(|dir| self + &dir.into())
    }

    pub fn length(&self) -> i32 {
        [self.dx.abs(), self.dy.abs(), (self.dx + self.dy).abs()]
            .into_iter()
            .max()
            .unwrap()
    }

    pub fn dist(&self, other: &LocDelta) -> i32 {
        (self - other).length()
    }
}

impl Add<&LocDelta> for &Loc {
    type Output = Loc;

    fn add(self, other: &LocDelta) -> Self::Output {
        Loc {
            x: self.x + other.dx,
            y: self.y + other.dy,
        }
    }
}

impl Sub<&LocDelta> for &Loc {
    type Output = Loc;

    fn sub(self, other: &LocDelta) -> Self::Output {
        Loc {
            x: self.x - other.dx,
            y: self.y - other.dy,
        }
    }
}

impl Sub<&Loc> for &Loc {
    type Output = LocDelta;

    fn sub(self, other: &Loc) -> Self::Output {
        LocDelta {
            dx: self.x - other.x,
            dy: self.y - other.y,
        }
    }
}

impl Add<&LocDelta> for &LocDelta {
    type Output = LocDelta;

    fn add(self, other: &LocDelta) -> Self::Output {
        LocDelta {
            dx: self.dx + other.dx,
            dy: self.dy + other.dy,
        }
    }
}

impl Sub<&LocDelta> for &LocDelta {
    type Output = LocDelta;

    fn sub(self, other: &LocDelta) -> Self::Output {
        LocDelta {
            dx: self.dx - other.dx,
            dy: self.dy - other.dy,
        }
    }
}

impl Neg for &LocDelta {
    type Output = LocDelta;

    fn neg(self) -> Self::Output {
        LocDelta {
            dx: -self.dx,
            dy: -self.dy,
        }
    }
}

enum Dir {
    W,
    NW,
    NE,
    E,
    SE,
    SW,
}

const DIRS: [Dir; 6] = [Dir::W, Dir::NW, Dir::NE, Dir::E, Dir::SE, Dir::SW];

impl From<Dir> for LocDelta {
    fn from(dir: Dir) -> Self {
        match dir {
            Dir::W => LocDelta { dx: -1, dy: 0 },
            Dir::NW => LocDelta { dx: -1, dy: 1 },
            Dir::NE => LocDelta { dx: 0, dy: 1 },
            Dir::E => LocDelta { dx: 1, dy: 0 },
            Dir::SE => LocDelta { dx: 1, dy: -1 },
            Dir::SW => LocDelta { dx: 0, dy: -1 },
        }
    }
}

type Path = Vec<LocDelta>;

lazy_static! {
    pub static ref PATH_MAPS: [HashMap<LocDelta, Vec<Path>>; 4] = {
        let mut path_maps = Vec::new();
        let mut hashmap = HashMap::new();
        hashmap.insert(LocDelta { dx: 0, dy: 0 }, vec![vec![]]);

        for i in 0..4 {
            path_maps.push(hashmap.clone());

            if i == 3 {
                break;
            }

            let mut next_map = hashmap.clone();
            for (delta, paths) in hashmap.iter() {
                for neighbor in delta.neighbors() {
                    for path in paths {
                        if path.len() >= 2 {
                            let prev_delta = &path[path.len() - 2];
                            if neighbor.dist(prev_delta) <= 1 {
                                continue;
                            }
                        }

                        let mut new_path = path.clone();
                        new_path.push(neighbor);
                        next_map.entry(neighbor).or_default().push(new_path);
                    }
                }
            }

            hashmap = next_map
        }

        path_maps.try_into().unwrap()
    };
}
