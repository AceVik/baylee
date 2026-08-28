//! Taiga — (no cost) — Land — Mountain Forest
//! Oracle: ({T}: Add {R} or {G}.)
//! Set: VMA #317 — Vintage Masters | Scryfall ID: 0c2c39fc-b564-4ab5-833c-ff029760b7a7 | Oracle ID: 22e3cf1d-3559-4ce1-954c-8dc815342979
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(165),
    oracle_id: "22e3cf1d-3559-4ce1-954c-8dc815342979",
    scryfall_id: "0c2c39fc-b564-4ab5-833c-ff029760b7a7",
    faces: &[FaceDef {
        name: "Taiga",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::MOUNTAIN, subtypes::land::FOREST],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Red]),
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
