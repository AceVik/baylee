//! Void Rend — {W}{U}{B} — Instant
//! Oracle: This spell can't be countered.
//! Oracle: Destroy target nonland permanent.
//! Set: SNC #230 — Streets of New Capenna | Scryfall ID: 2daab74d-d66b-4164-aa19-24e8d5536f7d | Oracle ID: 713f16db-95ec-479e-a48c-7a69f7668d7d
// IMPLEMENTED — uncounterable single-target destroy.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static NONLAND: Filter = Filter::LacksType(TypeSet::LAND);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(185),
    oracle_id: "713f16db-95ec-479e-a48c-7a69f7668d7d",
    scryfall_id: "2daab74d-d66b-4164-aa19-24e8d5536f7d",
    faces: &[FaceDef {
        name: "Void Rend",
        mana_cost: baylee_core::mana!("{W}{U}{B}"),
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
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue, Color::Black]),
    keywords: KeywordSet::UNCOUNTERABLE,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::Destroy {
            target: TargetSpec::Object(&NONLAND),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&NONLAND))),
    }],
};

#[cfg(test)]
mod tests {}
