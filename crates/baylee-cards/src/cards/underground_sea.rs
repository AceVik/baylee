//! Underground Sea — (no cost) — Land — Island Swamp
//! Oracle: ({T}: Add {U} or {B}.)
//! Set: VMA #323 — Vintage Masters | Scryfall ID: 26cee543-6eab-494e-a803-33a5d48d7d74 | Oracle ID: 4b22be3a-8ce1-47d1-b82e-6c3ccfb0548b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(178),
    oracle_id: "4b22be3a-8ce1-47d1-b82e-6c3ccfb0548b",
    scryfall_id: "26cee543-6eab-494e-a803-33a5d48d7d74",
    faces: &[FaceDef {
        name: "Underground Sea",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::ISLAND, subtypes::land::SWAMP],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
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
