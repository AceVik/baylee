//! Fracture — {W}{B} — Instant
//! Oracle: Destroy target artifact, enchantment, or planeswalker.
//! Set: SOC #310 — Secrets of Strixhaven Commander | Scryfall ID: cba33bf7-0919-408c-8eb0-0bb9fe920c81 | Oracle ID: f21d0319-0509-4ac1-b6e3-10955a26fd7a
// IMPLEMENTED — flexible destroy.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ARTIFACT_ENCHANTMENT_OR_WALKER: Filter = Filter::Or(&[
    Filter::HasType(TypeSet::ARTIFACT),
    Filter::HasType(TypeSet::ENCHANTMENT),
    Filter::HasType(TypeSet::PLANESWALKER),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(56),
    oracle_id: "f21d0319-0509-4ac1-b6e3-10955a26fd7a",
    scryfall_id: "cba33bf7-0919-408c-8eb0-0bb9fe920c81",
    faces: &[FaceDef {
        name: "Fracture",
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
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::Destroy {
            target: TargetSpec::Object(&ARTIFACT_ENCHANTMENT_OR_WALKER),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(
            &ARTIFACT_ENCHANTMENT_OR_WALKER,
        ))),
    }],
};

#[cfg(test)]
mod tests {}
