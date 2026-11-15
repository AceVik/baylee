//! The four tiers of the life workshop, with explicit costs and source lifetime.
use super::{Definition, Family, Lang, Phrase, Rarity};

pub(super) fn effect(definition: Definition, lang: Lang) -> String {
    let tier = usize::from(definition.rarity.tier() - 1);
    let mythic = definition.rarity == Rarity::MythicRare;
    let text = match definition.family {
        Family::SpringChalice => {
            Phrase::EchoesChaliceEffect.fill(lang, &[&[1, 2, 3, 5][tier].to_string()])
        }
        Family::Hearthstone => {
            Phrase::EchoesHearthEffect.fill(lang, &[&definition.rarity.tier().to_string()])
        }
        Family::BloodAmber => {
            let phrase = if mythic {
                Phrase::EchoesAmberMythicEffect
            } else {
                Phrase::EchoesAmberEffect
            };
            phrase.fill(lang, &[&[1, 2, 3, 3][tier].to_string()])
        }
        Family::LifeCoil => {
            let phrase = if mythic {
                Phrase::EchoesCoilMythicEffect
            } else {
                Phrase::EchoesCoilEffect
            };
            phrase.fill(lang, &[&[3, 2, 1, 1][tier].to_string()])
        }
        Family::Heartwood | Family::Satchel | Family::Compass | Family::Grove => {
            unreachable!("foundation text has its own reader")
        }
    };
    format!("{text} {}", Phrase::EchoesRemovableArtifact.text(lang))
}
