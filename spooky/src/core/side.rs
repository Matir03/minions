use anyhow::{anyhow, Result};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use super::convert::{FromIndex, ToIndex};
use std::ops::{Index, IndexMut, Not, Add};
use colored::Color;

/// Side/player in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum Side {
    Yellow,
    Blue,
}

impl Side {
    pub fn all() -> [Side; 2] {
        [Side::Yellow, Side::Blue]
    }

    pub fn sign(&self) -> i32 {
        match self {
            Side::Yellow => 1,
            Side::Blue => -1,
        }
    }

    pub fn opponent(self) -> Self {
        match self {
            Side::Yellow => Side::Blue,
            Side::Blue => Side::Yellow,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Side::Yellow => Color::Yellow,
            Side::Blue => Color::Blue,
        }
    }
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

impl Not for Side {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Side::Yellow => Side::Blue,
            Side::Blue => Side::Yellow,
        }
    }
}   

/// Array indexed by game side
#[derive(Debug, Clone, PartialEq, Eq)]
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

// impl<T: Add + Copy> Add for SideArray<T> {
//     type Output = SideArray<T::Output>;

//     fn add(self, rhs: Self) -> Self::Output {
//         SideArray {
//             values: [self.values[0] + rhs.values[0], self.values[1] + rhs.values[1]],
//         }
//     }
// }

impl<T> Index<Side> for SideArray<T> {
    type Output = T;

    fn index(&self, index: Side) -> &Self::Output {
        &self.values[index.to_index().unwrap()]
    }
}

impl<T> IndexMut<Side> for SideArray<T> {
    fn index_mut(&mut self, index: Side) -> &mut Self::Output {
        &mut self.values[index.to_index().unwrap()]
    }
}

impl<T: std::ops::AddAssign + Copy> std::ops::AddAssign for SideArray<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.values[0] += rhs.values[0];
        self.values[1] += rhs.values[1];
    }
}

impl<T: std::ops::AddAssign + Copy> std::ops::AddAssign<&Self> for SideArray<T> {
    fn add_assign(&mut self, rhs: &Self) {
        self.values[0] += rhs.values[0];
        self.values[1] += rhs.values[1];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_side_from_index() {
        assert_eq!(Side::from_index(0).unwrap(), Side::Yellow);
        assert_eq!(Side::from_index(1).unwrap(), Side::Blue);
        assert!(Side::from_index(2).is_err());
    }

    #[test]
    fn test_side_to_index() {
        assert_eq!(Side::Yellow.to_index().unwrap(), 0);
        assert_eq!(Side::Blue.to_index().unwrap(), 1);
    }

    #[test]
    fn test_side_array() {
        let mut array = SideArray::new(5, 10);
        
        // Test get
        assert_eq!(*array.get(Side::Yellow).unwrap(), 5);
        assert_eq!(*array.get(Side::Blue).unwrap(), 10);
        
        // Test get_mut
        *array.get_mut(Side::Yellow).unwrap() = 15;
        assert_eq!(*array.get(Side::Yellow).unwrap(), 15);
        
        // Test iter
        let values: Vec<_> = array.iter().copied().collect();
        assert_eq!(values, vec![15, 10]);
        
        // Test iter_mut
        for v in array.iter_mut() {
            *v *= 2;
        }
        assert_eq!(*array.get(Side::Yellow).unwrap(), 30);
        assert_eq!(*array.get(Side::Blue).unwrap(), 20);
    }
}
