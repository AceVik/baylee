//! Tundra — (no cost) — Land — Plains Island
//! Oracle: ({T}: Add {W} or {U}.)
//! Set: VMA #322 — Vintage Masters | Scryfall ID: efd35cb4-862d-4699-a197-b744989b3ceb | Oracle ID: 02418479-9455-417f-a6a1-004356faff37
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(175),
    oracle_id: "02418479-9455-417f-a6a1-004356faff37",
    scryfall_id: "efd35cb4-862d-4699-a197-b744989b3ceb",
    faces: &[FaceDef {
        name: "Tundra",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::ISLAND],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::White]),
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
