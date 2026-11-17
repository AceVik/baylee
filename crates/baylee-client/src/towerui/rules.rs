//! Localized original run rules. English parity is checked against the compiled DSL.
use baylee_client_core::{
    Lang, Phrase,
    echoes::domain::v2::catalog::{Definition, Family, Rarity},
};

pub(super) fn ability(item: Definition, index: usize, lang: Lang) -> Option<String> {
    if let Family::Runic(recipe) = item.family {
        return super::runic::ability(recipe, usize::from(item.rarity.tier() - 1), index, lang);
    }
    let token = baylee_cards::tokens::tower::run::definition(item.id())?;
    if index >= token.abilities.len() {
        return None;
    }
    let base = match item.family {
        Family::Heartwood | Family::Satchel | Family::Compass | Family::GildedAcorn => 0,
        Family::BloodAmber
        | Family::HarvestWheel
        | Family::RootReservoir
        | Family::TideglassLens
        | Family::StormSpindle => 2,
        _ => 1,
    };
    let raw = if index < base {
        primary(item, index, lang)
    } else {
        extra(item.family).get(index - base)?.text(lang).to_owned()
    };
    Some(super::words::strip_command_zone(&raw))
}

fn extra(family: Family) -> [Phrase; 3] {
    use Phrase as P;
    match family {
        Family::Runic(_) => unreachable!("runic abilities have their own shared metadata"),
        Family::Heartwood => [
            P::TowerRuleUpkeepLife,
            P::TowerRuleCreatureLife,
            P::TowerRuleLifeDrain,
        ],
        Family::Satchel | Family::TideglassLens => [
            P::TowerRuleUpkeepScry,
            P::TowerRuleLoot,
            P::TowerRuleDrawLife,
        ],
        Family::Compass => [
            P::TowerRuleUpkeepScry,
            P::TowerRuleFilter,
            P::TowerRuleFirstSpellDraw,
        ],
        Family::Grove => [
            P::TowerRuleTokenLife,
            P::TowerRuleTokensPlusOne,
            P::TowerRuleTokensTrample,
        ],
        Family::SpringChalice => [
            P::TowerRuleDrawLife,
            P::TowerRulePaidDraw,
            P::TowerRuleUpkeepDraw,
        ],
        Family::Hearthstone => [
            P::TowerRuleUpkeepPlant,
            P::TowerRuleTokensToughness,
            P::TowerRuleLifeDrain,
        ],
        Family::BloodAmber => [
            P::TowerRuleUpkeepCharge,
            P::TowerRuleDrawLife,
            P::TowerRuleLifeDrain,
        ],
        Family::LifeCoil => [
            P::TowerRuleDrawLife,
            P::TowerRuleUpkeepTwoLife,
            P::TowerRuleDrawDrain,
        ],
        Family::SeedCradle => [
            P::TowerRuleUpkeepLife,
            P::TowerRuleTokensToughness,
            P::TowerRuleTokensHaste,
        ],
        Family::SwarmBanner => [
            P::TowerRuleTokensVigilance,
            P::TowerRuleTokensTrample,
            P::TowerRuleTokensLifelink,
        ],
        Family::HarvestWheel => [
            P::TowerRuleDeathLife,
            P::TowerRuleUpkeepPlant,
            P::TowerRuleTokensHaste,
        ],
        Family::RootReservoir => [
            P::TowerRuleUpkeepPlant,
            P::TowerRuleDeathLife,
            P::TowerRuleTokensPlusOne,
        ],
        Family::EmberLantern => [
            P::TowerRuleUpkeepScry,
            P::TowerRuleSpellLife,
            P::TowerRuleSpellPlant,
        ],
        Family::GraveBell => [
            P::TowerRuleDeathLife,
            P::TowerRuleUpkeepPlant,
            P::TowerRuleDeathDraw,
        ],
        Family::StormSpindle => [
            P::TowerRuleUpkeepCharge,
            P::TowerRuleSpellLife,
            P::TowerRuleSpellPlant,
        ],
        Family::DawnAegis => [
            P::TowerRuleVigilance,
            P::TowerRuleLifelink,
            P::TowerRuleHexproof,
        ],
        Family::GildedAcorn => [
            P::TowerRuleUpkeepLife,
            P::TowerRuleUpkeepPlant,
            P::TowerRuleTokensPlusOne,
        ],
    }
}

fn primary(item: Definition, index: usize, lang: Lang) -> String {
    use Phrase as P;
    let tier = usize::from(item.rarity.tier() - 1);
    let number = item.rarity.tier().to_string();
    let say = |phrase: Phrase, args: &[&str]| phrase.fill(lang, args);
    match (item.family, index) {
        (Family::Grove, _) => say(P::TowerRulePlants, &[&number]),
        (Family::SpringChalice, _) => {
            say(P::TowerRuleGainUpkeep, &[&[1, 2, 3, 5][tier].to_string()])
        }
        (Family::Hearthstone, _) => say(P::TowerRuleGainCreature, &[&number]),
        (Family::BloodAmber, 0) => P::TowerRuleLifeCharge.text(lang).to_owned(),
        (Family::BloodAmber, _) => limited(say(P::TowerRuleDamage, &[&number]), item, lang, false),
        (Family::LifeCoil, _) => limited(
            say(
                P::TowerRuleDrawCost,
                &[&format!("{{{}}}", [3, 2, 1, 1][tier])],
            ),
            item,
            lang,
            false,
        ),
        (Family::SeedCradle, _) => say(
            P::TowerRuleCradle,
            &[
                &format!("{{{}}}", [3, 2, 1, 1][tier]),
                if tier == 3 { "2" } else { "1" },
            ],
        ),
        (Family::SwarmBanner, _) => say(
            P::TowerRuleBanner,
            &[
                &[1, 1, 2, 2][tier].to_string(),
                &[0, 1, 1, 2][tier].to_string(),
            ],
        ),
        (Family::HarvestWheel, 0) => P::TowerRuleDeathCharge.text(lang).to_owned(),
        (Family::HarvestWheel, _) => limited(
            say(
                P::TowerRuleStoredMana,
                &[
                    &[3, 2, 1, 1][tier].to_string(),
                    if tier == 3 { "{C}{C}{C}" } else { "{C}{C}" },
                ],
            ),
            item,
            lang,
            false,
        ),
        (Family::RootReservoir, 0) => P::TowerRuleTokenCharge.text(lang).to_owned(),
        (Family::RootReservoir, _) => limited(
            say(
                P::TowerRuleStoredMana,
                &[&[3, 2, 2, 1][tier].to_string(), "{G}"],
            ),
            item,
            lang,
            true,
        ),
        (Family::EmberLantern, _) => say(P::TowerRuleSpellDrain, &[&number]),
        (Family::TideglassLens, 0) => P::TowerRuleDrawCharge.text(lang).to_owned(),
        (Family::TideglassLens, _) => say(P::TowerRuleScry, &[&number]),
        (Family::GraveBell, _) => say(P::TowerRuleDeathDrain, &[&number]),
        (Family::StormSpindle, 0) => P::TowerRuleSpellCharge.text(lang).to_owned(),
        (Family::StormSpindle, _) => limited(
            say(P::TowerRuleStoredAny, &[&[3, 2, 1, 1][tier].to_string()]),
            item,
            lang,
            false,
        ),
        (Family::DawnAegis, _) => say(P::TowerRuleAegis, &[&number]),
        _ => unreachable!("families without base abilities are handled above"),
    }
}

fn limited(mut text: String, item: Definition, lang: Lang, rare_unlimited: bool) -> String {
    if item.rarity != Rarity::MythicRare && !(rare_unlimited && item.rarity == Rarity::Rare) {
        text.push(' ');
        text.push_str(Phrase::TowerRuleOnce.text(lang));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    // Numeric spelling and singular/plural are editorial, and redundant
    // command-zone boilerplate is intentionally omitted on Tower UI surfaces.
    // Every other word, cost, timing clause and per-turn limit must match the
    // executable description.
    fn canonical(text: &str) -> String {
        super::super::words::strip_command_zone(text)
            .to_lowercase()
            .replace(" a ", " 1 ")
            .replace("two", "2")
            .replace("three", "3")
            .replace("four", "4")
            .replace("counters", "counter")
            .replace("tokens", "token")
    }

    #[test]
    fn every_run_ability_has_matching_english_complete_german_and_no_command_zone_boilerplate() {
        for family in Family::ALL {
            for rarity in Rarity::ALL {
                let item = Definition { family, rarity };
                let token = baylee_cards::tokens::tower::run::definition(item.id()).unwrap();
                for index in 0..token.abilities.len() {
                    let native =
                        baylee_cards::tokens::tower::run::ability_text(token, index as u32)
                            .unwrap();
                    let english = ability(item, index, Lang::En).unwrap();
                    assert_eq!(
                        canonical(&english),
                        canonical(native),
                        "{item:?} ability {index}"
                    );
                    assert!(
                        !english.to_lowercase().contains("command zone"),
                        "{item:?} ability {index} retained command-zone boilerplate: {english}"
                    );
                    let german = ability(item, index, Lang::De).unwrap();
                    assert!(
                        !german.is_empty() && german != english,
                        "{item:?} ability {index}"
                    );
                    assert!(
                        !german.to_lowercase().contains("kommandozone"),
                        "{item:?} ability {index} retained Kommandozone boilerplate: {german}"
                    );
                }
                assert!(ability(item, token.abilities.len(), Lang::De).is_none());
            }
        }
    }
}
