//! Shared localized runic names/rules; the numeric balance comes from native data.
use baylee_client_core::{
    Lang, Phrase as P,
    echoes::domain::v2::catalog::{RunicRecipe, Sigil, Vessel},
};

pub(super) fn name(recipe: RunicRecipe, lang: Lang) -> String {
    P::RunicName.fill(
        lang,
        &[
            vessel(recipe.vessel).text(lang),
            sigil(recipe.sigil).text(lang),
        ],
    )
}

const fn vessel(vessel: Vessel) -> P {
    match vessel {
        Vessel::Chalice => P::RunicChalice,
        Vessel::Scepter => P::RunicScepter,
        Vessel::Prism => P::RunicPrism,
        Vessel::Seed => P::RunicSeed,
        Vessel::Tome => P::RunicTome,
        Vessel::Horn => P::RunicHorn,
        Vessel::Flask => P::RunicFlask,
        Vessel::Bulwark => P::RunicBulwark,
        Vessel::Banner => P::RunicBanner,
        Vessel::Lens => P::RunicLens,
    }
}

const fn sigil(sigil: Sigil) -> P {
    match sigil {
        Sigil::Dawn => P::RunicDawn,
        Sigil::Dusk => P::RunicDusk,
        Sigil::Root => P::RunicRoot,
        Sigil::Hearth => P::RunicHearth,
        Sigil::Bloom => P::RunicBloom,
        Sigil::Ash => P::RunicAsh,
        Sigil::Ember => P::RunicEmber,
        Sigil::Lore => P::RunicLore,
        Sigil::Pulse => P::RunicPulse,
        Sigil::Forge => P::RunicForge,
        Sigil::Veil => P::RunicVeil,
        Sigil::Valor => P::RunicValor,
        Sigil::Fang => P::RunicFang,
        Sigil::Echo => P::RunicEcho,
        Sigil::Wraith => P::RunicWraith,
        Sigil::Star => P::RunicStar,
    }
}

const fn event(sigil: Sigil) -> P {
    match sigil {
        Sigil::Dawn => P::RunicEventDawn,
        Sigil::Dusk => P::RunicEventDusk,
        Sigil::Root => P::RunicEventRoot,
        Sigil::Hearth => P::RunicEventHearth,
        Sigil::Bloom => P::RunicEventBloom,
        Sigil::Ash => P::RunicEventAsh,
        Sigil::Ember => P::RunicEventEmber,
        Sigil::Lore => P::RunicEventLore,
        Sigil::Pulse => P::RunicEventPulse,
        Sigil::Forge => P::RunicEventForge,
        Sigil::Veil => P::RunicEventVeil,
        Sigil::Valor => P::RunicEventValor,
        Sigil::Fang => P::RunicEventFang,
        Sigil::Echo => P::RunicEventEcho,
        Sigil::Wraith => P::RunicEventWraith,
        Sigil::Star => P::RunicEventStar,
    }
}

fn reward(vessel: Vessel, n: u16, lang: Lang) -> String {
    match vessel {
        Vessel::Chalice => P::RunicRewardLife,
        Vessel::Scepter => P::RunicRewardDrain,
        Vessel::Prism => P::RunicRewardScry,
        Vessel::Seed => P::RunicRewardPlant,
        Vessel::Tome => P::RunicRewardDraw,
        Vessel::Horn => P::RunicRewardSoldier,
        Vessel::Flask => P::RunicRewardMana,
        Vessel::Bulwark => P::RunicRewardGuard,
        Vessel::Banner => P::RunicRewardPower,
        Vessel::Lens => P::RunicRewardLoot,
    }
    .fill(lang, &[&n.to_string()])
}

const fn bonus(vessel: Vessel) -> P {
    match vessel {
        Vessel::Chalice | Vessel::Tome => P::TowerRuleDrawLife,
        Vessel::Scepter => P::RunicAttackDrain,
        Vessel::Prism => P::TowerRuleUpkeepScry,
        Vessel::Seed => P::TowerRuleTokensVigilance,
        Vessel::Horn => P::TowerRuleTokensToughness,
        Vessel::Flask => P::TowerRuleSpellLife,
        Vessel::Bulwark => P::TowerRuleVigilance,
        Vessel::Banner => P::TowerRuleTokensHaste,
        Vessel::Lens => P::RunicDiscardLife,
    }
}

pub(super) fn ability(
    recipe: RunicRecipe,
    tier: usize,
    index: usize,
    lang: Lang,
) -> Option<String> {
    let level = recipe.level(tier)?;
    if index >= 2 + tier {
        return None;
    }
    Some(match index {
        0 => P::RunicCharge.fill(
            lang,
            &[event(recipe.sigil).text(lang), &level.gathered.to_string()],
        ),
        1 => {
            let mana = if level.mana == 0 {
                String::new()
            } else {
                format!("{{{}}}, ", level.mana)
            };
            let mut text = super::words::strip_command_zone(&P::RunicActivate.fill(
                lang,
                &[
                    &mana,
                    &level.charges.to_string(),
                    &reward(recipe.vessel, level.amount, lang),
                ],
            ));
            if level.activations != 0 {
                text.push(' ');
                text.push_str(&if level.activations == 1 {
                    P::RunicOnce.text(lang).to_owned()
                } else {
                    P::RunicLimit.fill(lang, &[&level.activations.to_string()])
                });
            }
            text
        }
        2 => P::TowerRuleUpkeepCharge.text(lang).into(),
        3 => bonus(recipe.vessel).text(lang).into(),
        4 => P::RunicLegend.fill(lang, &[&reward(recipe.vessel, level.amount, lang)]),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runic_charge_and_activate_use_fluent_plurals_in_english_and_german() {
        assert_eq!(
            P::RunicCharge.fill(Lang::En, &["Whenever you gain life", "1"]),
            "Whenever you gain life, put 1 charge counter on this item."
        );
        assert_eq!(
            P::RunicCharge.fill(Lang::En, &["Whenever you gain life", "2"]),
            "Whenever you gain life, put 2 charge counters on this item."
        );
        assert_eq!(
            P::RunicCharge.fill(Lang::De, &["Immer wenn du Lebenspunkte dazuerhältst", "1"]),
            "Immer wenn du Lebenspunkte dazuerhältst, lege 1 Ladungsmarke auf diesen Gegenstand."
        );
        assert_eq!(
            P::RunicCharge.fill(Lang::De, &["Immer wenn du Lebenspunkte dazuerhältst", "2"]),
            "Immer wenn du Lebenspunkte dazuerhältst, lege 2 Ladungsmarken auf diesen Gegenstand."
        );

        assert_eq!(
            P::RunicActivate.fill(Lang::En, &["", "1", "Draw a card"]),
            "Remove 1 charge counter from this item: Draw a card. Activate from the command zone."
        );
        assert_eq!(
            P::RunicActivate.fill(Lang::En, &["{1}, ", "3", "Draw a card"]),
            "{1}, Remove 3 charge counters from this item: Draw a card. Activate from the command zone."
        );
        assert_eq!(
            P::RunicActivate.fill(Lang::De, &["", "1", "Ziehe eine Karte"]),
            "Entferne 1 Ladungsmarke von diesem Gegenstand: Ziehe eine Karte. Aktiviere dies aus der Kommandozone."
        );
        assert_eq!(
            P::RunicActivate.fill(Lang::De, &["{1}, ", "3", "Ziehe eine Karte"]),
            "{1}, Entferne 3 Ladungsmarken von diesem Gegenstand: Ziehe eine Karte. Aktiviere dies aus der Kommandozone."
        );
    }
}
