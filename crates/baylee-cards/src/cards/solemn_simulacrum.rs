//! Solemn Simulacrum — {4} — Artifact Creature — Golem
//! Oracle: When this creature enters, you may search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.
//! Oracle: When this creature dies, you may draw a card.
//! Set: MSC #215 — Marvel Super Heroes Commander | Scryfall ID: daafd816-f7c1-4630-9e5c-a1e5db570a35 | Oracle ID: 00c0543c-2a1f-4425-8283-4062d74a1637
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(151),
    oracle_id: "00c0543c-2a1f-4425-8283-4062d74a1637",
    scryfall_id: "daafd816-f7c1-4630-9e5c-a1e5db570a35",
    faces: &[FaceDef {
        name: "Solemn Simulacrum",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::creature::GOLEM],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::EMPTY,
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
