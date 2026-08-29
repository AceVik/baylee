//! Vanishing Verse — {W}{B} — Instant
//! Oracle: Exile target monocolored permanent.
//! Set: SOC #335 — Secrets of Strixhaven Commander | Scryfall ID: 8a475868-a335-45e7-9d59-9dc4c2cea1ae | Oracle ID: 5b8f0cdf-572d-4025-b930-79291f7c35be
// IMPLEMENTED — monocolored exile removal.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static MONOCOLORED_PERMANENT: Filter = Filter::Monocolored;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(180),
    oracle_id: "5b8f0cdf-572d-4025-b930-79291f7c35be",
    scryfall_id: "8a475868-a335-45e7-9d59-9dc4c2cea1ae",
    faces: &[FaceDef {
        name: "Vanishing Verse",
        mana_cost: baylee_core::mana!("{W}{B}"),
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
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::Exile {
            target: TargetSpec::Object(&MONOCOLORED_PERMANENT),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&MONOCOLORED_PERMANENT))),
    }],
};

#[cfg(test)]
mod tests {}
