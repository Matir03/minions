use std::{
    collections::HashMap, fmt::Display, ops::{Add, Neg, Sub}, str::FromStr
};
use anyhow::Context;
use lazy_static::lazy_static;

pub const GRID_LEN: usize = 10;
pub const GRID_SIZE: usize = GRID_LEN * GRID_LEN;

/// A location on the game board
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Loc {
    pub x: i32,
    pub y: i32,
}

impl Loc {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn in_bounds(&self) -> bool {
        self.x >= 0 && self.x < GRID_LEN as i32 &&
        self.y >= 0 && self.y < GRID_LEN as i32 
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

    pub fn index(&self) -> usize {
        (self.y as usize) * GRID_LEN + (self.x as usize)
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
        let (x, y) = s.split_once(',')
            .context("Invalid loc")?;

        Ok(Loc {
            x: x.parse()?,
            y: y.parse()?,
        })
    }
}

impl Display for Loc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{}", self.x, self.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Delta {
    pub dx: i32,
    pub dy: i32,
}

impl Delta {
    pub fn neighbors(&self) -> [Delta; 6] {
        DIRS.map(|dir| self + &dir.into())
    }

    pub fn length(&self) -> i32 {
        [self.dx.abs(), self.dy.abs(), (self.dx + self.dy).abs()]
            .into_iter()
            .max()
            .unwrap()
    }

    pub fn dist(&self, other: &Delta) -> i32 {
        (self - other).length()
    }
}

impl Add<&Delta> for &Loc {
    type Output = Loc;

    fn add(self, other: &Delta) -> Self::Output {
        Loc {
            x: self.x + other.dx,
            y: self.y + other.dy,
        }
    }
}

impl Sub<&Delta> for &Loc {
    type Output = Loc;

    fn sub(self, other: &Delta) -> Self::Output {
        Loc {
            x: self.x - other.dx,
            y: self.y - other.dy,
        }
    }
}

impl Sub<&Loc> for &Loc {
    type Output = Delta;

    fn sub(self, other: &Loc) -> Self::Output {
        Delta {
            dx: self.x - other.x,
            dy: self.y - other.y,
        }
    }
}

impl Add<&Delta> for &Delta {
    type Output = Delta;

    fn add(self, other: &Delta) -> Self::Output {
        Delta {
            dx: self.dx + other.dx,
            dy: self.dy + other.dy,
        }
    }
}

impl Sub<&Delta> for &Delta {
    type Output = Delta;

    fn sub(self, other: &Delta) -> Self::Output {
        Delta {
            dx: self.dx - other.dx,
            dy: self.dy - other.dy,
        }
    }
}

impl Neg for &Delta {
    type Output = Delta;

    fn neg(self) -> Self::Output {
        Delta {
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

const DIRS: [Dir; 6] = [
    Dir::W, 
    Dir::NW, 
    Dir::NE, 
    Dir::E, 
    Dir::SE,
    Dir::SW,
];

impl From<Dir> for Delta {
    fn from(dir: Dir) -> Self {
        match dir {
            Dir::W => Delta { dx: -1, dy: 0 },
            Dir::NW => Delta { dx: -1, dy: 1 },
            Dir::NE => Delta { dx: 0, dy: 1 },
            Dir::E => Delta { dx: 1, dy: 0 },
            Dir::SE => Delta { dx: 1, dy: -1 },
            Dir::SW => Delta { dx: 0, dy: -1 },
        }
    }
} 

type Path = Vec<Delta>;

lazy_static!(
    pub static ref PATH_MAPS: [HashMap<Delta, Vec<Path>>; 4] = {
        let mut path_maps = Vec::new();
        let mut hashmap = HashMap::new();
        hashmap.insert(Delta { dx: 0, dy: 0 }, vec![vec![]]);

        for i in 0..4 {
            path_maps.push(hashmap.clone());

            if i == 3 { break; }

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
);