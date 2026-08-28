//! Sunken Hollow — (no cost) — Land — Island Swamp
//! Oracle: ({T}: Add {U} or {B}.)
//! Oracle: This land enters tapped unless you control two or more basic lands.
//! Set: MSC #271 — Marvel Super Heroes Commander | Scryfall ID: 3a8eef9b-9b03-42cd-a27a-07021bf0b33f | Oracle ID: cd2c90ac-2b04-461c-92f3-939871b6b6a3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(159),
    oracle_id: "cd2c90ac-2b04-461c-92f3-939871b6b6a3",
    scryfall_id: "3a8eef9b-9b03-42cd-a27a-07021bf0b33f",
    faces: &[FaceDef {
        name: "Sunken Hollow",
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
