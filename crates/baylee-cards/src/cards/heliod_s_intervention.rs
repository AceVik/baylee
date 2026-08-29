//! Heliod's Intervention — {X}{W}{W} — Instant
//! Oracle: Choose one —
//! Oracle: • Destroy X target artifacts and/or enchantments.
//! Oracle: • Target player gains twice X life.
//! Set: OTC #81 — Outlaws of Thunder Junction Commander | Scryfall ID: 9519bb3a-bed3-48e8-93ae-9e9b2e7d646a | Oracle ID: e7564d66-767c-4cd9-a5f0-0f2488a4a74b
// IMPLEMENTED — both modes (X-target destroy / 2X lifegain).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    SpellMode, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ARTIFACT_OR_ENCHANTMENT: Filter = Filter::Or(&[
    Filter::HasType(TypeSet::ARTIFACT),
    Filter::HasType(TypeSet::ENCHANTMENT),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(67),
    oracle_id: "e7564d66-767c-4cd9-a5f0-0f2488a4a74b",
    scryfall_id: "9519bb3a-bed3-48e8-93ae-9e9b2e7d646a",
    faces: &[FaceDef {
        name: "Heliod's Intervention",
        mana_cost: baylee_core::mana!("{X}{W}{W}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalSpell {
        modes: &[
            SpellMode {
                effects: &[Effect::Destroy {
                    target: TargetSpec::Object(&ARTIFACT_OR_ENCHANTMENT),
                }],
                target: Some(TargetSpec::Object(&ARTIFACT_OR_ENCHANTMENT)),
                cost_override: None,
            },
            SpellMode {
                effects: &[Effect::GainLifeDoubleX],
                target: None,
                cost_override: None,
            },
        ],
    }],
};

#[cfg(test)]
mod tests {}
