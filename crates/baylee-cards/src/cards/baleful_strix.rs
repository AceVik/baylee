//! Baleful Strix — {U}{B} — Artifact Creature — Bird
//! Oracle: Flying, deathtouch
//! Oracle: When this creature enters, draw a card.
//! Set: OTC #215 — Outlaws of Thunder Junction Commander | Scryfall ID: be8439e6-f779-49f0-806a-b04995697a6a | Oracle ID: 37688720-03de-4eca-a82d-a0afe8d58adc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(10),
    oracle_id: "37688720-03de-4eca-a82d-a0afe8d58adc",
    scryfall_id: "be8439e6-f779-49f0-806a-b04995697a6a",
    faces: &[FaceDef {
        name: "Baleful Strix",
        mana_cost: baylee_core::mana!("{U}{B}"),
        types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::BIRD],
        power: Some(1),
        toughness: Some(1),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
