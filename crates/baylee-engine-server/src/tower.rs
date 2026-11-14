//! Tower's compiled native pack. All effects use the ordinary rules pipeline.
use baylee_cards::tokens::tower::{
    TOWER_GROVE_COMMON, TOWER_GROVE_MYTHIC, TOWER_GROVE_RARE, TOWER_GROVE_UNCOMMON,
};
use baylee_engine::{
    extensions::{NativeDefinition, NativeRulePack},
    state::SetupError,
};
use baylee_tower::v2::{
    self,
    catalog::{Definition, Family, Rarity},
};

#[cfg(feature = "tower")]
pub mod verification;

/// Statically bound revision-two pack; no per-event dispatch or service callback.
#[derive(Clone, Copy, Debug)]
pub struct Pack;

impl NativeRulePack for Pack {
    const PACK: u16 = v2::PACK;
    const REVISION: u16 = v2::REVISION;

    fn validate(preset: &baylee_core::preset::GamePreset) -> Result<(), SetupError> {
        v2::loadout::validate(preset).map_err(|_| SetupError::UnsupportedRulePack)
    }

    fn definition(id: u16) -> Option<NativeDefinition> {
        let definition = Definition::from_id(id)?;
        if definition.family != Family::Grove {
            return Some(NativeDefinition::SetupOnly);
        }
        Some(NativeDefinition::Permanent(match definition.rarity {
            Rarity::Common => &TOWER_GROVE_COMMON,
            Rarity::Uncommon => &TOWER_GROVE_UNCOMMON,
            Rarity::Rare => &TOWER_GROVE_RARE,
            Rarity::MythicRare => &TOWER_GROVE_MYTHIC,
        }))
    }
}

#[cfg(test)]
mod tests;
