//! Tower's compiled native pack. All effects use the ordinary rules pipeline.
use baylee_cards::tokens::tower::{
    GROWTH_RELICS, LIFE_RELICS, TOWER_GROVE_COMMON, TOWER_GROVE_MYTHIC, TOWER_GROVE_RARE,
    TOWER_GROVE_UNCOMMON,
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
pub mod durable;
#[cfg(feature = "tower")]
pub mod verification;

/// Retained generation-three rules for regression tests and development probes.
/// Production recovery still requires the exact archived executable identity.
#[derive(Clone, Copy, Debug)]
pub struct Pack;

/// Independent run equipment, with registry-verified Commander admission.
/// The current compiled Tower executor; no dynamic dispatch or service callback.
#[derive(Clone, Copy, Debug)]
pub struct RunPack;

impl NativeRulePack for RunPack {
    const PACK: u16 = v2::PACK;
    const REVISION: u16 = 5;

    fn validate(preset: &baylee_core::preset::GamePreset) -> Result<(), SetupError> {
        v2::loadout::validate_for(preset, v2::generation::RUN)
            .map_err(|_| SetupError::UnsupportedRulePack)?;
        for seat in &preset.seats {
            baylee_tower_content::validate_seat(seat)
                .map_err(|_| SetupError::UnsupportedRulePack)?;
        }
        Ok(())
    }

    fn definition(id: u16) -> Option<NativeDefinition> {
        baylee_cards::tokens::tower::run::definition(id).map(NativeDefinition::Emblem)
    }
}

impl NativeRulePack for Pack {
    const PACK: u16 = v2::PACK;
    const REVISION: u16 = 4;

    fn validate(preset: &baylee_core::preset::GamePreset) -> Result<(), SetupError> {
        v2::loadout::validate_for(preset, 3).map_err(|_| SetupError::UnsupportedRulePack)
    }

    fn definition(id: u16) -> Option<NativeDefinition> {
        let definition = Definition::from_id(id)?;
        let tier = usize::from(definition.rarity.tier() - 1);
        Some(match definition.family {
            Family::Heartwood | Family::Satchel | Family::Compass => NativeDefinition::SetupOnly,
            Family::Grove => NativeDefinition::Permanent(match definition.rarity {
                Rarity::Common => &TOWER_GROVE_COMMON,
                Rarity::Uncommon => &TOWER_GROVE_UNCOMMON,
                Rarity::Rare => &TOWER_GROVE_RARE,
                Rarity::MythicRare => &TOWER_GROVE_MYTHIC,
            }),
            Family::SpringChalice => NativeDefinition::Permanent(LIFE_RELICS[0][tier]),
            Family::Hearthstone => NativeDefinition::Permanent(LIFE_RELICS[1][tier]),
            Family::BloodAmber => NativeDefinition::Permanent(LIFE_RELICS[2][tier]),
            Family::LifeCoil => NativeDefinition::Permanent(LIFE_RELICS[3][tier]),
            Family::SeedCradle => NativeDefinition::Permanent(GROWTH_RELICS[0][tier]),
            Family::SwarmBanner => NativeDefinition::Permanent(GROWTH_RELICS[1][tier]),
            Family::HarvestWheel => NativeDefinition::Permanent(GROWTH_RELICS[2][tier]),
            Family::RootReservoir => NativeDefinition::Permanent(GROWTH_RELICS[3][tier]),
            Family::EmberLantern
            | Family::TideglassLens
            | Family::GraveBell
            | Family::StormSpindle
            | Family::DawnAegis
            | Family::GildedAcorn => return None,
        })
    }
}

#[cfg(test)]
mod tests;
