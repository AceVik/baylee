//! Raffine's Tower — (no cost) — Land — Plains Island Swamp
//! Oracle: ({T}: Add {W}, {U}, or {B}.)
//! Oracle: This land enters tapped.
//! Oracle: Cycling {3} ({3}, Discard this card: Draw a card.)
//! Set: SNC #254 — Streets of New Capenna | Scryfall ID: a2c56479-4bee-4edb-80d7-4af010b7c793 | Oracle ID: 6e9ef5ef-6aed-4d3e-a59b-9e3dc8740b1b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(122),
    oracle_id: "6e9ef5ef-6aed-4d3e-a59b-9e3dc8740b1b",
    scryfall_id: "a2c56479-4bee-4edb-80d7-4af010b7c793",
    faces: &[FaceDef {
        name: "Raffine's Tower",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::land::PLAINS,
            subtypes::land::ISLAND,
            subtypes::land::SWAMP,
        ],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
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
