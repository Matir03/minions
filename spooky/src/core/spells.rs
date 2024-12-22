use super::{
    side::Side,
    board::Board,
    units::Unit,
    loc::Loc,
};
use anyhow::Result;
use enum_variant_type::EnumVariantType;

/// Types of spells that can be cast
#[derive(Debug, Clone, Hash, PartialEq, Eq, EnumVariantType)]
pub enum Spell {
    Shield,
    Reposition,
    // TODO: Add other spell types
}

#[derive(Debug, Clone, EnumVariantType)]
pub enum SpellCast {
    CastShield {
        target: Loc,
    },
    CastReposition {
        from_sq: Loc,
        to_sq: Loc,
    }
}

impl SpellCast {
    pub fn cast(&self, board: &mut Board, caster: Side) -> Result<()> {
        todo!();
    }
}

