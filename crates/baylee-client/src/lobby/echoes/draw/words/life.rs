//! The four tiers of the life workshop, with explicit costs and source lifetime.
use super::{Definition, Family, Lang, Phrase, Rarity};

pub(super) fn effect(definition: Definition, lang: Lang) -> String {
    let value = amount(definition).to_string();
    let mythic = definition.rarity == Rarity::MythicRare;
    let text = match definition.family {
        Family::SpringChalice => Phrase::EchoesChaliceEffect.fill(lang, &[&value]),
        Family::Hearthstone => Phrase::EchoesHearthEffect.fill(lang, &[&value]),
        Family::BloodAmber => {
            let phrase = if mythic {
                Phrase::EchoesAmberMythicEffect
            } else {
                Phrase::EchoesAmberEffect
            };
            phrase.fill(lang, &[&value])
        }
        Family::LifeCoil => {
            let phrase = if mythic {
                Phrase::EchoesCoilMythicEffect
            } else {
                Phrase::EchoesCoilEffect
            };
            phrase.fill(lang, &[&value])
        }
        Family::Heartwood
        | Family::Satchel
        | Family::Compass
        | Family::Grove
        | Family::SeedCradle
        | Family::SwarmBanner
        | Family::HarvestWheel
        | Family::RootReservoir
        | Family::EmberLantern
        | Family::TideglassLens
        | Family::GraveBell
        | Family::StormSpindle
        | Family::DawnAegis
        | Family::GildedAcorn => {
            unreachable!("foundation text has its own reader")
        }
    };
    format!("{text} {}", Phrase::EchoesRemovableArtifact.text(lang))
}

pub(super) fn tier(definition: Definition, lang: Lang) -> String {
    let mythic = definition.rarity == Rarity::MythicRare;
    let phrase = match definition.family {
        Family::SpringChalice => Phrase::EchoesChaliceTier,
        Family::Hearthstone => Phrase::EchoesHearthTier,
        Family::BloodAmber if mythic => Phrase::EchoesAmberMythicTier,
        Family::BloodAmber => Phrase::EchoesAmberTier,
        Family::LifeCoil if mythic => Phrase::EchoesCoilMythicTier,
        Family::LifeCoil => Phrase::EchoesCoilTier,
        Family::Heartwood
        | Family::Satchel
        | Family::Compass
        | Family::Grove
        | Family::SeedCradle
        | Family::SwarmBanner
        | Family::HarvestWheel
        | Family::RootReservoir
        | Family::EmberLantern
        | Family::TideglassLens
        | Family::GraveBell
        | Family::StormSpindle
        | Family::DawnAegis
        | Family::GildedAcorn => {
            unreachable!("foundation text has its own reader")
        }
    };
    phrase.fill(lang, &[&amount(definition).to_string()])
}

fn amount(definition: Definition) -> u8 {
    let tier = usize::from(definition.rarity.tier() - 1);
    match definition.family {
        Family::SpringChalice => [1, 2, 3, 5][tier],
        Family::Hearthstone => definition.rarity.tier(),
        Family::BloodAmber => [1, 2, 3, 3][tier],
        Family::LifeCoil => [3, 2, 1, 1][tier],
        Family::Heartwood
        | Family::Satchel
        | Family::Compass
        | Family::Grove
        | Family::SeedCradle
        | Family::SwarmBanner
        | Family::HarvestWheel
        | Family::RootReservoir
        | Family::EmberLantern
        | Family::TideglassLens
        | Family::GraveBell
        | Family::StormSpindle
        | Family::DawnAegis
        | Family::GildedAcorn => {
            unreachable!("foundation text has its own reader")
        }
    }
}
