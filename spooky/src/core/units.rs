/// Type of attack a unit can perform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attack {
    Damage(i32),
    Unsummon,
    Deathtouch,
}

/// Represents a unit type in the game
#[derive(Debug, Clone)]
pub struct Unit {
    pub attack: Attack,
    pub num_attack: i32,
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

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use anyhow::{anyhow, Result};
use super::convert::{FromIndex, ToIndex};

/// Labels for different unit types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive, ToPrimitive)]
pub enum UnitLabel {
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
    TerrainNecromancer
}

impl UnitLabel {
    /// Convert a unit label to its FEN character representation
    pub fn to_fen_char(self) -> char {
        match self {
            UnitLabel::Zombie => 'Z',
            UnitLabel::Initiate => 'I',
            UnitLabel::Skeleton => 'S',
            UnitLabel::Serpent => 'R',
            UnitLabel::Warg => 'W',
            UnitLabel::Ghost => 'G',
            UnitLabel::Wight => 'T',
            UnitLabel::Haunt => 'H',
            UnitLabel::Shrieker => 'K',
            UnitLabel::Spectre => 'P',
            UnitLabel::Rat => 'A',
            UnitLabel::Sorcerer => 'C',
            UnitLabel::Witch => 'W',
            UnitLabel::Vampire => 'V',
            UnitLabel::Mummy => 'M',
            UnitLabel::Lich => 'L',
            UnitLabel::Void => 'O',
            UnitLabel::Cerberus => 'E',
            UnitLabel::Wraith => 'R',
            UnitLabel::Horror => 'H',
            UnitLabel::Banshee => 'B',
            UnitLabel::Elemental => 'E',
            UnitLabel::Harpy => 'Y',
            UnitLabel::Shadowlord => 'D',
            UnitLabel::BasicNecromancer => 'N',
            UnitLabel::ArcaneNecromancer => 'N',
            UnitLabel::RangedNecromancer => 'N',
            UnitLabel::MountedNecromancer => 'N',
            UnitLabel::DeadlyNecromancer => 'N',
            UnitLabel::BattleNecromancer => 'N',
            UnitLabel::ZombieNecromancer => 'N',
            UnitLabel::ManaNecromancer => 'N',
            UnitLabel::TerrainNecromancer => 'N',
        }
    }

    /// Convert a FEN character to its unit label
    pub fn from_fen_char(c: char) -> Option<Self> {
        // Convert to uppercase for matching
        match c.to_ascii_uppercase() {
            'Z' => Some(UnitLabel::Zombie),
            'I' => Some(UnitLabel::Initiate),
            'S' => Some(UnitLabel::Skeleton),
            'R' => Some(UnitLabel::Serpent),
            'W' => Some(UnitLabel::Warg),
            'G' => Some(UnitLabel::Ghost),
            'T' => Some(UnitLabel::Wight),
            'H' => Some(UnitLabel::Haunt),
            'K' => Some(UnitLabel::Shrieker),
            'P' => Some(UnitLabel::Spectre),
            'A' => Some(UnitLabel::Rat),
            'C' => Some(UnitLabel::Sorcerer),
            'V' => Some(UnitLabel::Vampire),
            'M' => Some(UnitLabel::Mummy),
            'L' => Some(UnitLabel::Lich),
            'O' => Some(UnitLabel::Void),
            'B' => Some(UnitLabel::Banshee),
            'E' => Some(UnitLabel::Elemental),
            'Y' => Some(UnitLabel::Harpy),
            'D' => Some(UnitLabel::Shadowlord),
            'N' => Some(UnitLabel::BasicNecromancer), // Default to basic necromancer
            _ => None,
        }
    }
}

impl FromIndex for UnitLabel {
    fn from_index(idx: usize) -> Result<Self> {
        FromPrimitive::from_usize(idx)
            .ok_or_else(|| anyhow!("Invalid unit index: {}", idx))
    }
}

impl ToIndex for UnitLabel {
    fn to_index(&self) -> Result<usize> {
        ToPrimitive::to_usize(self)
            .ok_or_else(|| anyhow!("Invalid unit label"))
    }
}

const NUM_UNITS: usize = 33;

const UNIT_STATS: [Unit; NUM_UNITS] = [
    // Zombie
    Unit {
        attack: Attack::Damage(1),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(2),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(5),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(3),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(1),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(1),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(1),
        num_attack: 5,
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
    Unit {
        attack: Attack::Damage(2),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(3),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(1),
        num_attack: 1,
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
    Unit {
        attack: Attack::Deathtouch,
        num_attack: 1,
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
    Unit {
        attack: Attack::Unsummon,
        num_attack: 6,
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
    Unit {
        attack: Attack::Damage(1),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(2),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(1),
        num_attack: 6,
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
    Unit {
        attack: Attack::Deathtouch,
        num_attack: 1,
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
    Unit {
        attack: Attack::Unsummon,
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(1),
        num_attack: 2,
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
    Unit {
        attack: Attack::Damage(1),
        num_attack: 3,
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
    Unit {
        attack: Attack::Damage(1),
        num_attack: 10,
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
    Unit {
        attack: Attack::Deathtouch,
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(3),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(2),
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(8),
        num_attack: 1,
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
    Unit {
        attack: Attack::Unsummon,
        num_attack: 1,
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
    Unit {
        attack: Attack::Unsummon,
        num_attack: 1,
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
    Unit {
        attack: Attack::Unsummon,
        num_attack: 1,
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
    Unit {
        attack: Attack::Unsummon,
        num_attack: 1,
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
    Unit {
        attack: Attack::Deathtouch,
        num_attack: 1,
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
    Unit {
        attack: Attack::Damage(1),
        num_attack: 3,
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
    Unit {
        attack: Attack::Unsummon,
        num_attack: 1,
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
    Unit {
        attack: Attack::Unsummon,
        num_attack: 1,
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
    Unit {
        attack: Attack::Unsummon,
        num_attack: 1,
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
        assert_eq!(UnitLabel::from_index(0).unwrap().to_index().unwrap(), 0);
        
        // Test all unit labels have unique indices
        let mut indices = Vec::new();
        for i in 0..33 {
            if let Ok(label) = UnitLabel::from_index(i) {
                let idx = label.to_index().unwrap();
                assert!(!indices.contains(&idx), "Duplicate index: {}", idx);
                indices.push(idx);
            }
        }
        
        // Test invalid index
        assert!(UnitLabel::from_index(100).is_err());
    }

    #[test]
    fn test_unit_fen_chars() {
        // Test basic unit label to FEN char conversion
        assert_eq!(UnitLabel::Zombie.to_fen_char(), 'Z');
        assert_eq!(UnitLabel::Initiate.to_fen_char(), 'I');
        
        // Test FEN char to unit label conversion
        assert_eq!(UnitLabel::from_fen_char('Z').unwrap(), UnitLabel::Zombie);
        assert_eq!(UnitLabel::from_fen_char('I').unwrap(), UnitLabel::Initiate);
        
        // Test case insensitivity
        assert_eq!(UnitLabel::from_fen_char('z').unwrap(), UnitLabel::Zombie);
        assert_eq!(UnitLabel::from_fen_char('i').unwrap(), UnitLabel::Initiate);
        
        // Test invalid char
        assert!(UnitLabel::from_fen_char('X').is_none());
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
        let unit = Unit {
            cost: 100,
            num_attack: 1,
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
        assert_eq!(unit.num_attack, 1);
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
