//! Shared item names, separated rarity/limit metadata, and command-zone-free
//! rules text for the workshop, table drawer, stack, and game log.
use std::collections::HashMap;
use std::sync::LazyLock;

use baylee_cards_dsl::{AbilityDef, ActivationLimit};
use baylee_client_core::{
    echoes::domain,
    i18n::{Lang, Phrase},
};
use domain::v2::{
    catalog::{Definition, Family, Potion, Rarity},
    shop::Goods,
};

/// Removes redundant `"Activate from the command zone."` / `"Aktiviere dies aus
/// der Kommandozone."` boilerplate: every Tower run item already lives in the
/// Run Equipment zone, while explicit per-turn limits remain intact.
pub(super) fn strip_command_zone(text: &str) -> String {
    let mut out = text
        .replace(" Activate from the command zone.", "")
        .replace("Activate from the command zone.", "")
        .replace(" from the command zone.", ".")
        .replace(" Aktiviere dies aus der Kommandozone.", "")
        .replace("Aktiviere dies aus der Kommandozone.", "")
        .replace(" Aus der Kommandozone aktivierbar.", "")
        .replace("Aus der Kommandozone aktivierbar.", "")
        .replace(" aus der Kommandozone aktivierbar.", " aktivierbar.");
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

/// strips any legacy `" · Common"` .. `" · Mythic Rare"` suffix if a caller
/// passed a combined string, returning only the clean primary item title.
pub(crate) fn clean_title(raw: &str) -> &str {
    // The legacy suffix was always English; a localized one is stripped too.
    let legacy = ["Common", "Uncommon", "Rare", "Mythic Rare"];
    let localized = [Lang::En, Lang::De]
        .into_iter()
        .flat_map(|lang| Rarity::ALL.map(|r| rarity(r, lang)));
    for label in legacy.into_iter().chain(localized) {
        if let Some(prefix) = raw.strip_suffix(label).and_then(|s| s.strip_suffix(" · ")) {
            return prefix;
        }
    }
    raw
}

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

/// Primary item display name (never mashes `" · Rarity"` onto the title; render
/// rarity as a secondary subline via [`rarity`] or [`rarity_subline`]).
pub(crate) fn goods(goods: Goods, lang: Lang) -> String {
    name(goods, lang)
}

/// A rarity's name in the interface's language (the deck builder's words).
pub(crate) fn rarity(rarity: Rarity, lang: Lang) -> &'static str {
    match rarity {
        Rarity::Common => Phrase::RarityCommon,
        Rarity::Uncommon => Phrase::RarityUncommon,
        Rarity::Rare => Phrase::RarityRare,
        Rarity::MythicRare => Phrase::RarityMythic,
    }
    .text(lang)
}

/// Secondary rarity subline for an item's detail header (`None` for ordinary
/// single-use potions and card offers that carry no relic tier).
#[must_use]
pub(crate) fn rarity_subline(goods: Goods, lang: Lang) -> Option<&'static str> {
    match goods {
        Goods::Relic(definition) => Some(rarity(definition.rarity, lang)),
        Goods::Mythic => Some(rarity(Rarity::MythicRare, lang)),
        Goods::Potion(_) | Goods::Card(_) => None,
    }
}

/// Authoritative per-turn activation cap read straight from the compiled
/// `TokenDef` (`None` when unlimited, such as Potions or unlimited Mythics).
#[must_use]
pub(crate) fn activation_limit(goods: Goods) -> Option<u8> {
    let Goods::Relic(definition) = goods else {
        return None;
    };
    let token = baylee_cards::tokens::tower::run::definition(definition.id())?;
    for ability in token.abilities {
        let limit = match ability {
            AbilityDef::Activated { limit, .. }
            | AbilityDef::ActivatedConditional { limit, .. }
            | AbilityDef::ModalActivated { limit, .. } => *limit,
            _ => continue,
        };
        if let ActivationLimit::PerTurn(n) = limit
            && n > 0
        {
            return Some(n);
        }
    }
    None
}

/// Compact localized pill badge (`"1× pro Zug"` / `"Once per turn"`) shown ONLY
/// on items with an actual per-turn activation cap.
#[must_use]
pub(crate) fn activation_limit_tag(goods: Goods, lang: Lang) -> Option<String> {
    let count = activation_limit(goods)?;
    Some(match (lang, count) {
        (Lang::De, 1) => "1× pro Zug".to_owned(),
        (Lang::De, n) => format!("{n}× pro Zug"),
        (Lang::En, 1) => "Once per turn".to_owned(),
        (Lang::En, n) => format!("{n}× per turn"),
    })
}

/// Resolves a localized English or German Tower item display name back to a
/// default `Goods` entry (Common tier for relics) for text-only log lookups
/// when a structured token ID is absent.
#[allow(dead_code)] // Fallback door for text-only HUD/log item lookups.
#[must_use]
pub(crate) fn lookup_by_name(text: &str) -> Option<Goods> {
    static BY_NAME: LazyLock<HashMap<String, Goods>> = LazyLock::new(|| {
        let mut map = HashMap::new();
        for lang in [Lang::En, Lang::De] {
            for potion in [Potion::Healing, Potion::Mana] {
                let g = Goods::Potion(potion);
                map.insert(name(g, lang).to_lowercase(), g);
            }
            map.insert(name(Goods::Mythic, lang).to_lowercase(), Goods::Mythic);
            for family in Family::ALL {
                let g = Goods::Relic(Definition {
                    family,
                    rarity: Rarity::Common,
                });
                map.entry(name(g, lang).to_lowercase()).or_insert(g);
            }
        }
        map
    });
    BY_NAME
        .get(&clean_title(text.trim()).to_lowercase())
        .copied()
}

pub(crate) fn name(goods: Goods, lang: Lang) -> String {
    match goods {
        Goods::Card(id) => card(id, lang),
        Goods::Relic(Definition { family, .. }) => match family {
            Family::Runic(recipe) => return super::runic::name(recipe, lang),
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

/// Every run ability uses the same localized copy on the shelf and at the table.
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
        if let Some(text) = super::rules::ability(definition, index, lang) {
            lines.push(text);
        }
    }
    lines.join("\n")
}

pub(crate) fn token_ability(token: u16, index: u32, lang: Lang) -> Option<String> {
    match crate::itemart::identity::run_token(token)? {
        Goods::Relic(item) => super::rules::ability(item, usize::try_from(index).ok()?, lang),
        Goods::Potion(potion) if index == 0 => Some(strip_command_zone(
            match potion {
                Potion::Healing => Phrase::TowerRuleHealing,
                Potion::Mana => Phrase::TowerRuleMana,
            }
            .text(lang),
        )),
        _ => None,
    }
}
