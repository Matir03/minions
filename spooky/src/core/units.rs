/// Type of attack a unit can perform
use std::str::FromStr;

use crate::core::tech::NUM_TECHS;

use super::convert::{FromIndex, ToIndex};
use anyhow::{anyhow, ensure, Context, Result};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attack {
    Damage(i32),
    Unsummon,
    Deathtouch,
}

impl Attack {
    pub fn damage_value(&self) -> i32 {
        match self {
            Attack::Damage(d) => *d,
            Attack::Unsummon => 0,
            Attack::Deathtouch => 0,
        }
    }
}

/// Represents a unit type in the game
#[derive(Debug, Clone)]
pub struct UnitStats {
    pub attack: Attack,
    pub num_attacks: i32,
    pub defense: i32,
    pub speed: i32,
    pub range: i32,
    pub cost: i32,
    pub rebate: i32,
    pub necromancer: bool,
    pub lumbering: bool,
    pub flying: bool,
    pub persistent: bool,
    pub spawn: bool,
    pub blink: bool,
}

/// Labels for different unit types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, FromPrimitive, ToPrimitive)]
pub enum Unit {
    Zombie,
    Initiate,
    Skeleton,
    Serpent,
    Warg,
    Ghost,
    Wight,
    Haunt,
    Shrieker,
    Spectre,
    Rat,
    Sorcerer,
    Witch,
    Vampire,
    Mummy,
    Lich,
    Void,
    Cerberus,
    Wraith,
    Horror,
    Banshee,
    Elemental,
    Harpy,
    Shadowlord,
    BasicNecromancer,
    ArcaneNecromancer,
    RangedNecromancer,
    MountedNecromancer,
    DeadlyNecromancer,
    BattleNecromancer,
    ZombieNecromancer,
    ManaNecromancer,
    TerrainNecromancer,
}

impl Unit {
    pub const NUM_UNITS: usize = 33;
    /// Convert a unit label to its FEN character representation
    pub fn to_fen_char(self) -> char {
        match self {
            Unit::Zombie => 'Z',
            Unit::Initiate => 'I',
            Unit::Skeleton => 'S',
            Unit::Serpent => 'P',
            Unit::Warg => 'W',
            Unit::Ghost => 'G',
            Unit::Wight => 'T',
            Unit::Haunt => 'H',
            Unit::Shrieker => 'K',
            Unit::Spectre => 'X',
            Unit::Rat => 'A',
            Unit::Sorcerer => 'U',
            Unit::Witch => 'J',
            Unit::Vampire => 'V',
            Unit::Mummy => 'M',
            Unit::Lich => 'L',
            Unit::Void => 'O',
            Unit::Cerberus => 'C',
            Unit::Wraith => 'R',
            Unit::Horror => 'Q',
            Unit::Banshee => 'B',
            Unit::Elemental => 'E',
            Unit::Harpy => 'Y',
            Unit::Shadowlord => 'D',
            Unit::BasicNecromancer => 'N',
            Unit::ArcaneNecromancer => 'N',
            Unit::RangedNecromancer => 'N',
            Unit::MountedNecromancer => 'N',
            Unit::DeadlyNecromancer => 'N',
            Unit::BattleNecromancer => 'N',
            Unit::ZombieNecromancer => 'N',
            Unit::ManaNecromancer => 'N',
            Unit::TerrainNecromancer => 'N',
        }
    }

    /// Convert a FEN character to its unit label
    pub fn from_fen_char(c: char) -> Option<Self> {
        // Convert to uppercase for matching
        match c.to_ascii_uppercase() {
            'Z' => Some(Unit::Zombie),
            'I' => Some(Unit::Initiate),
            'S' => Some(Unit::Skeleton),
            'P' => Some(Unit::Serpent),
            'W' => Some(Unit::Warg),
            'G' => Some(Unit::Ghost),
            'T' => Some(Unit::Wight),
            'H' => Some(Unit::Haunt),
            'K' => Some(Unit::Shrieker),
            'X' => Some(Unit::Spectre),
            'A' => Some(Unit::Rat),
            'U' => Some(Unit::Sorcerer),
            'J' => Some(Unit::Witch),
            'V' => Some(Unit::Vampire),
            'M' => Some(Unit::Mummy),
            'L' => Some(Unit::Lich),
            'O' => Some(Unit::Void),
            'C' => Some(Unit::Cerberus),
            'R' => Some(Unit::Wraith),
            'Q' => Some(Unit::Horror),
            'B' => Some(Unit::Banshee),
            'E' => Some(Unit::Elemental),
            'Y' => Some(Unit::Harpy),
            'D' => Some(Unit::Shadowlord),
            'N' => Some(Unit::BasicNecromancer), // Default to basic necromancer
            _ => None,
        }
    }

    pub fn stats(&self) -> &UnitStats {
        &UNIT_STATS[*self as usize]
    }

    pub fn counters(&self) -> Vec<Unit> {
        let mut counters = Vec::new();
        let self_index = self.to_index().unwrap();
        if self_index + 1 < NUM_TECHS {
            counters.push(self_index + 1);
        }
        if self_index + 2 < NUM_TECHS {
            counters.push(self_index + 2);
        }
        if self_index >= 4 {
            counters.push(self_index - 3);
        }
        counters
            .into_iter()
            .map(Unit::from_index)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    pub fn anticounters(&self) -> Vec<Unit> {
        let mut anticounters = Vec::new();
        let self_index = self.to_index().unwrap();
        if self_index > 0 {
            anticounters.push(self_index - 1);
        }
        if self_index > 1 {
            anticounters.push(self_index - 2);
        }
        if self_index + 3 < NUM_TECHS {
            anticounters.push(self_index + 3);
        }
        anticounters
            .into_iter()
            .map(Unit::from_index)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_fen_char())
    }
}

impl FromStr for Unit {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ensure!(s.len() == 1, format!("Invalid unit label: {}", s));

        Unit::from_fen_char(s.chars().next().unwrap()).context(format!("Invalid unit label: {}", s))
    }
}

impl FromIndex for Unit {
    fn from_index(idx: usize) -> Result<Self> {
        FromPrimitive::from_usize(idx).ok_or_else(|| anyhow!("Invalid unit index: {}", idx))
    }
}

impl ToIndex for Unit {
    fn to_index(&self) -> Result<usize> {
        ToPrimitive::to_usize(self).ok_or_else(|| anyhow!("to_index failed for unit: {}", self))
    }
}

const UNIT_STATS: [UnitStats; Unit::NUM_UNITS] = [
    // Zombie
    UnitStats {
        attack: Attack::Damage(1),
        num_attacks: 1,
        defense: 1,
        speed: 1,
        range: 1,
        cost: 2,
        rebate: 0,
        necromancer: false,
        lumbering: true,
        flying: false,
        persistent: false,
        spawn: false,
        blink: false,
    },
    // Initiate
    UnitStats {
        attack: Attack::Damage(2),
        num_attacks: 1,
        defense: 3,
        speed: 2,
        range: 1,
        cost: 2,
        rebate: 0,
        necromancer: false,
        lumbering: true,
        flying: false,
        persistent: false,
        spawn: true,
        blink: false,
    },
    // Skeleton
    UnitStats {
        attack: Attack::Damage(5),
        num_attacks: 1,
        defense: 2,
        speed: 1,
        range: 1,
        cost: 4,
        rebate: 2,
        necromancer: false,
        lumbering: false,
        flying: true,
        persistent: false,
        spawn: false,
        blink: false,
    },
    // Serpent
    UnitStats {
        attack: Attack::Damage(3),
        num_attacks: 1,
        defense: 1,
        speed: 2,
        range: 1,
        cost: 5,
        rebate: 3,
        necromancer: false,
        lumbering: false,
        flying: true,
        persistent: false,
        spawn: false,
        blink: false,
    },
    // Warg
    UnitStats {
        attack: Attack::Damage(1),
        num_attacks: 1,
        defense: 1,
        speed: 3,
        range: 1,
        cost: 3,
        rebate: 1,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: false,
        spawn: false,
        blink: false,
    },
    // Ghost
    UnitStats {
        attack: Attack::Damage(1),
        num_attacks: 1,
        defense: 4,
        speed: 1,
        range: 1,
        cost: 3,
        rebate: 1,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: true,
        spawn: false,
        blink: false,
    },
    // Wight
    UnitStats {
        attack: Attack::Damage(1),
        num_attacks: 5,
        defense: 3,
        speed: 0,
        range: 2,
        cost: 4,
        rebate: 1,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: false,
        spawn: false,
        blink: true,
    },
    // Haunt
    UnitStats {
        attack: Attack::Damage(2),
        num_attacks: 1,
        defense: 2,
        speed: 0,
        range: 3,
        cost: 5,
        rebate: 1,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: true,
        spawn: false,
        blink: true,
    },
    // Shrieker
    UnitStats {
        attack: Attack::Damage(3),
        num_attacks: 1,
        defense: 1,
        speed: 3,
        range: 1,
        cost: 6,
        rebate: 4,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: false,
        spawn: false,
        blink: false,
    },
    // Spectre
    UnitStats {
        attack: Attack::Damage(1),
        num_attacks: 1,
        defense: 5,
        speed: 2,
        range: 1,
        cost: 5,
        rebate: 3,
        necromancer: false,
        lumbering: false,
        flying: true,
        persistent: false,
        spawn: false,
        blink: false,
    },
    // Rat
    UnitStats {
        attack: Attack::Deathtouch,
        num_attacks: 1,
        defense: 1,
        speed: 1,
        range: 1,
        cost: 2,
        rebate: 1,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: false,
        spawn: true,
        blink: false,
    },
    // Sorcerer
    UnitStats {
        attack: Attack::Unsummon,
        num_attacks: 6,
        defense: 3,
        speed: 2,
        range: 1,
        cost: 4,
        rebate: 0,
        necromancer: false,
        lumbering: false,
        flying: true,
        persistent: false,
        spawn: false,
        blink: true,
    },
    // Witch
    UnitStats {
        attack: Attack::Damage(1),
        num_attacks: 1,
        defense: 1,
        speed: 1,
        range: 3,
        cost: 3,
        rebate: 0,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: false,
        spawn: false,
        blink: false,
    },
    // Vampire
    UnitStats {
        attack: Attack::Damage(2),
        num_attacks: 1,
        defense: 6,
        speed: 1,
        range: 1,
        cost: 4,
        rebate: 2,
        necromancer: false,
        lumbering: false,
        flying: true,
        persistent: true,
        spawn: false,
        blink: false,
    },
    // Mummy
    UnitStats {
        attack: Attack::Damage(1),
        num_attacks: 6,
        defense: 6,
        speed: 1,
        range: 1,
        cost: 6,
        rebate: 4,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: false,
        spawn: true,
        blink: false,
    },
    // Lich
    UnitStats {
        attack: Attack::Deathtouch,
        num_attacks: 1,
        defense: 2,
        speed: 1,
        range: 2,
        cost: 6,
        rebate: 4,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: false,
        spawn: false,
        blink: false,
    },
    // Void
    UnitStats {
        attack: Attack::Unsummon,
        num_attacks: 1,
        defense: 2,
        speed: 2,
        range: 2,
        cost: 5,
        rebate: 3,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: false,
        spawn: false,
        blink: true,
    },
    // Cerberus
    UnitStats {
        attack: Attack::Damage(1),
        num_attacks: 2,
        defense: 3,
        speed: 3,
        range: 1,
        cost: 6,
        rebate: 4,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: true,
        spawn: false,
        blink: false,
    },
    // Wraith
    UnitStats {
        attack: Attack::Damage(1),
        num_attacks: 3,
        defense: 8,
        speed: 2,
        range: 2,
        cost: 6,
        rebate: 3,
        necromancer: false,
        lumbering: true,
        flying: true,
        persistent: true,
        spawn: true,
        blink: false,
    },
    // Horror
    UnitStats {
        attack: Attack::Damage(1),
        num_attacks: 10,
        defense: 6,
        speed: 1,
        range: 1,
        cost: 3,
        rebate: 0,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: false,
        spawn: false,
        blink: false,
    },
    // Banshee
    UnitStats {
        attack: Attack::Deathtouch,
        num_attacks: 1,
        defense: 2,
        speed: 2,
        range: 1,
        cost: 5,
        rebate: 2,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: true,
        spawn: false,
        blink: true,
    },
    // Elemental
    UnitStats {
        attack: Attack::Damage(3),
        num_attacks: 1,
        defense: 2,
        speed: 1,
        range: 3,
        cost: 8,
        rebate: 5,
        necromancer: false,
        lumbering: false,
        flying: false,
        persistent: false,
        spawn: false,
        blink: false,
    },
    // Harpy
    UnitStats {
        attack: Attack::Damage(2),
        num_attacks: 1,
        defense: 4,
        speed: 2,
        range: 2,
        cost: 7,
        rebate: 4,
        necromancer: false,
        lumbering: false,
        flying: true,
        persistent: false,
        spawn: true,
        blink: false,
    },
    // Shadowlord
    UnitStats {
        attack: Attack::Damage(8),
        num_attacks: 1,
        defense: 10,
        speed: 2,
        range: 1,
        cost: 8,
        rebate: 3,
        necromancer: false,
        lumbering: false,
        flying: true,
        persistent: true,
        spawn: false,
        blink: false,
    },
    // Basic Necromancer
    UnitStats {
        attack: Attack::Unsummon,
        num_attacks: 1,
        defense: 7,
        speed: 1,
        range: 1,
        cost: 0,
        rebate: 0,
        necromancer: true,
        lumbering: false,
        flying: false,
        persistent: true,
        spawn: true,
        blink: false,
    },
    // Arcane Necromancer
    UnitStats {
        attack: Attack::Unsummon,
        num_attacks: 1,
        defense: 10,
        speed: 1,
        range: 1,
        cost: 0,
        rebate: 0,
        necromancer: true,
        lumbering: false,
        flying: false,
        persistent: true,
        spawn: true,
        blink: false,
    },
    // Ranged Necromancer
    UnitStats {
        attack: Attack::Unsummon,
        num_attacks: 1,
        defense: 7,
        speed: 1,
        range: 2,
        cost: 0,
        rebate: 0,
        necromancer: true,
        lumbering: false,
        flying: false,
        persistent: true,
        spawn: true,
        blink: false,
    },
    // Mounted Necromancer
    UnitStats {
        attack: Attack::Unsummon,
        num_attacks: 1,
        defense: 10,
        speed: 2,
        range: 0,
        cost: 0,
        rebate: 0,
        necromancer: true,
        lumbering: true,
        flying: true,
        persistent: true,
        spawn: true,
        blink: false,
    },
    // Deadly Necromancer
    UnitStats {
        attack: Attack::Deathtouch,
        num_attacks: 1,
        defense: 10,
        speed: 1,
        range: 1,
        cost: 0,
        rebate: 0,
        necromancer: true,
        lumbering: false,
        flying: false,
        persistent: true,
        spawn: true,
        blink: false,
    },
    // Battle Necromancer
    UnitStats {
        attack: Attack::Damage(1),
        num_attacks: 3,
        defense: 10,
        speed: 1,
        range: 1,
        cost: 0,
        rebate: 0,
        necromancer: true,
        lumbering: false,
        flying: false,
        persistent: true,
        spawn: true,
        blink: false,
    },
    // Zombie Necromancer
    UnitStats {
        attack: Attack::Unsummon,
        num_attacks: 1,
        defense: 10,
        speed: 1,
        range: 1,
        cost: 0,
        rebate: 0,
        necromancer: true,
        lumbering: true,
        flying: false,
        persistent: true,
        spawn: true,
        blink: false,
    },
    // Mana Necromancer
    UnitStats {
        attack: Attack::Unsummon,
        num_attacks: 1,
        defense: 10,
        speed: 1,
        range: 1,
        cost: 0,
        rebate: 0,
        necromancer: true,
        lumbering: false,
        flying: false,
        persistent: true,
        spawn: true,
        blink: false,
    },
    // Terrain Necromancer
    UnitStats {
        attack: Attack::Unsummon,
        num_attacks: 1,
        defense: 10,
        speed: 1,
        range: 1,
        cost: 0,
        rebate: 0,
        necromancer: true,
        lumbering: false,
        flying: false,
        persistent: true,
        spawn: true,
        blink: false,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_label_conversion() {
        // Test basic conversions
        assert_eq!(Unit::from_index(0).unwrap().to_index().unwrap(), 0);

        // Test all unit labels have unique indices
        let mut indices = Vec::new();
        for i in 0..33 {
            if let Ok(label) = Unit::from_index(i) {
                let idx = label.to_index().unwrap();
                assert!(!indices.contains(&idx), "Duplicate index: {}", idx);
                indices.push(idx);
            }
        }

        // Test invalid index
        assert!(Unit::from_index(100).is_err());
    }

    #[test]
    fn test_unit_fen_chars() {
        // Test basic unit label to FEN char conversion
        assert_eq!(Unit::Zombie.to_fen_char(), 'Z');
        assert_eq!(Unit::Initiate.to_fen_char(), 'I');

        // Test FEN char to unit label conversion
        assert_eq!(Unit::from_fen_char('Z').unwrap(), Unit::Zombie);
        assert_eq!(Unit::from_fen_char('I').unwrap(), Unit::Initiate);

        // Test case insensitivity
        assert_eq!(Unit::from_fen_char('z').unwrap(), Unit::Zombie);
        assert_eq!(Unit::from_fen_char('i').unwrap(), Unit::Initiate);

        // Test that 'X' maps to Spectre
        assert_eq!(Unit::from_fen_char('X').unwrap(), Unit::Spectre);

        // Test invalid char
        assert!(Unit::from_fen_char('@').is_none());
    }

    #[test]
    fn test_attack() {
        let attack = Attack::Damage(3);

        match attack {
            Attack::Damage(damage) => assert_eq!(damage, 3),
            _ => panic!("Expected Damage attack"),
        }
    }

    #[test]
    fn test_unit() {
        let unit = UnitStats {
            cost: 100,
            num_attacks: 1,
            defense: 10,
            speed: 2,
            range: 1,
            rebate: 0,
            necromancer: false,
            lumbering: false,
            flying: false,
            persistent: true,
            spawn: true,
            blink: false,
            attack: Attack::Damage(2),
        };

        assert_eq!(unit.cost, 100);
        assert_eq!(unit.num_attacks, 1);
        assert_eq!(unit.defense, 10);
        assert_eq!(unit.speed, 2);
        assert_eq!(unit.range, 1);
        assert_eq!(unit.rebate, 0);
        assert!(!unit.necromancer);
        assert!(!unit.lumbering);
        assert!(!unit.flying);
        assert!(unit.persistent);
        assert!(unit.spawn);
        assert!(!unit.blink);
        match unit.attack {
            Attack::Damage(damage) => assert_eq!(damage, 2),
            _ => panic!("Expected Damage attack"),
        }
    }
}
