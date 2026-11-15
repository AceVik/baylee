//! Original token-workshop text: own tokens, optional payments and real charge costs.
use super::{Definition, Family, Lang, Phrase, Rarity};

pub(super) fn effect(definition: Definition, lang: Lang) -> String {
    let event = match definition.family {
        Family::SeedCradle => Phrase::EchoesCradleEvent.text(lang),
        Family::HarvestWheel => Phrase::EchoesHarvestEvent.text(lang),
        Family::RootReservoir => Phrase::EchoesRootsEvent.text(lang),
        Family::SwarmBanner => "",
        Family::Heartwood
        | Family::Satchel
        | Family::Compass
        | Family::Grove
        | Family::SpringChalice
        | Family::Hearthstone
        | Family::BloodAmber
        | Family::LifeCoil => {
            unreachable!("other workshops have their own text")
        }
    };
    format!(
        "{}{} {}",
        if event.is_empty() {
            String::new()
        } else {
            format!("{event} ")
        },
        tier(definition, lang),
        Phrase::EchoesRemovableArtifact.text(lang)
    )
}

pub(super) fn tier(definition: Definition, lang: Lang) -> String {
    let at = usize::from(definition.rarity.tier() - 1);
    let mythic = definition.rarity == Rarity::MythicRare;
    match definition.family {
        Family::SeedCradle => {
            let mana = [3, 2, 1, 1][at].to_string();
            if mythic {
                Phrase::EchoesCradleTwoTier
            } else {
                Phrase::EchoesCradleTier
            }
            .fill(lang, &[&mana])
        }
        Family::SwarmBanner => {
            let (power, toughness) = [(1, 0), (1, 1), (2, 1), (2, 2)][at];
            let text =
                Phrase::EchoesBannerTier.fill(lang, &[&power.to_string(), &toughness.to_string()]);
            if mythic {
                format!("{text} {}", Phrase::EchoesBannerTrample.text(lang))
            } else {
                text
            }
        }
        Family::HarvestWheel => mana_text(
            [3, 2, 1, 1][at],
            if mythic { 3 } else { 2 },
            !mythic,
            Phrase::EchoesColorlessMana.text(lang),
            lang,
        ),
        Family::RootReservoir => mana_text(
            [3, 2, 2, 1][at],
            1,
            at < 2,
            Phrase::EchoesGreenMana.text(lang),
            lang,
        ),
        Family::Heartwood
        | Family::Satchel
        | Family::Compass
        | Family::Grove
        | Family::SpringChalice
        | Family::Hearthstone
        | Family::BloodAmber
        | Family::LifeCoil => {
            unreachable!("other workshops have their own text")
        }
    }
}

fn mana_text(charges: u8, mana: u8, tap: bool, color: &str, lang: Lang) -> String {
    let phrase = if tap {
        Phrase::EchoesStoredManaTier
    } else {
        Phrase::EchoesStoredManaFreeTier
    };
    phrase.fill_count(lang, usize::from(charges), &[&mana.to_string(), color])
}
