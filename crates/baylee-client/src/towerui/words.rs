//! Shared item names and rules for the workshop and table drawer.
use baylee_client_core::{
    echoes::domain,
    i18n::{Lang, Phrase},
};
use domain::v2::{
    catalog::{Definition, Family, Potion, Rarity},
    shop::Goods,
};

pub(crate) fn description(goods: Goods, lang: Lang) -> String {
    match goods {
        Goods::Relic(definition) => run_effect(definition, lang),
        Goods::Potion(Potion::Healing) => Phrase::EchoesHealingEffect.text(lang).into(),
        Goods::Potion(Potion::Mana) => Phrase::EchoesManaEffect.text(lang).into(),
        Goods::Mythic => Phrase::EchoesMythicEffect.text(lang).into(),
        Goods::Card(_) => Phrase::EchoesCardEffect.text(lang).into(),
    }
}

pub(crate) fn card(id: baylee_core::ids::CardIndex, lang: Lang) -> String {
    baylee_cards::by_index(id).map_or_else(
        || Phrase::EchoesUnknownCard.text(lang).to_string(),
        |card| card.name().to_string(),
    )
}

pub(crate) fn goods(goods: Goods, lang: Lang) -> String {
    if let Goods::Relic(definition) = goods {
        format!("{} · {}", name(goods, lang), rarity(definition.rarity))
    } else {
        name(goods, lang)
    }
}

pub(crate) const fn rarity(rarity: Rarity) -> &'static str {
    match rarity {
        Rarity::Common => "Common",
        Rarity::Uncommon => "Uncommon",
        Rarity::Rare => "Rare",
        Rarity::MythicRare => "Mythic Rare",
    }
}

pub(crate) fn name(goods: Goods, lang: Lang) -> String {
    match goods {
        Goods::Card(id) => card(id, lang),
        Goods::Relic(Definition { family, .. }) => match family {
            Family::Heartwood => Phrase::EchoesHeartwood,
            Family::Satchel => Phrase::EchoesSatchel,
            Family::Compass => Phrase::EchoesCompass,
            Family::Grove => Phrase::EchoesGrove,
            Family::SpringChalice => Phrase::EchoesChalice,
            Family::Hearthstone => Phrase::EchoesHearth,
            Family::BloodAmber => Phrase::EchoesAmber,
            Family::LifeCoil => Phrase::EchoesCoil,
            Family::SeedCradle => Phrase::EchoesCradle,
            Family::SwarmBanner => Phrase::EchoesBanner,
            Family::HarvestWheel => Phrase::EchoesHarvest,
            Family::RootReservoir => Phrase::EchoesRoots,
            Family::EmberLantern => Phrase::EchoesLantern,
            Family::TideglassLens => Phrase::EchoesLens,
            Family::GraveBell => Phrase::EchoesBell,
            Family::StormSpindle => Phrase::EchoesSpindle,
            Family::DawnAegis => Phrase::EchoesAegis,
            Family::GildedAcorn => Phrase::EchoesAcorn,
        }
        .text(lang)
        .to_owned(),
        Goods::Potion(Potion::Healing) => Phrase::EchoesHealing.text(lang).to_string(),
        Goods::Potion(Potion::Mana) => Phrase::EchoesMana.text(lang).to_string(),
        Goods::Mythic => Phrase::EchoesMythic.text(lang).to_string(),
    }
}

/// Generation-five native sentences; the redesigned shop adds localized detail rows.
pub(crate) fn run_effect(definition: Definition, lang: Lang) -> String {
    let Some(token) = baylee_cards::tokens::tower::run::definition(definition.id()) else {
        return String::new();
    };
    let mut lines = Vec::new();
    if let Ok(stats) = domain::v2::catalog::run_attributes(&[definition]) {
        match definition.family {
            Family::Heartwood => {
                lines.push(Phrase::EchoesLifeEffect.fill(lang, &[&(stats.life - 20).to_string()]));
            }
            Family::Satchel => lines.push(Phrase::EchoesCardsEffect.fill(
                lang,
                &[
                    &(stats.cards - 7).to_string(),
                    &(stats.maximum_hand - 7).to_string(),
                ],
            )),
            Family::Compass => lines.push(
                Phrase::EchoesMulliganEffect.fill(lang, &[&(stats.free_mulligans - 1).to_string()]),
            ),
            _ => {}
        }
    }
    if definition.family == Family::GildedAcorn {
        lines.push(Phrase::EchoesAcornEffect.fill(lang, &[&definition.rarity.tier().to_string()]));
        if definition.rarity == Rarity::MythicRare {
            lines.push(Phrase::EchoesAcornMythic.text(lang).to_owned());
        }
    }
    for index in 0..token.abilities.len() {
        if let Some(text) = baylee_cards::tokens::tower::run::ability_text(
            token,
            u32::try_from(index).expect("bounded native abilities"),
        ) {
            lines.push(text.to_owned());
        }
    }
    lines.join("\n")
}
