//! Swamp — (no cost) — Basic Land — Swamp
//! Oracle: ({T}: Add {B}.)
//! Set: TRK #321 — Star Trek | Scryfall ID: b7387103-1df1-4fd0-9e91-1544509792c7 | Oracle ID: 56719f6a-1a6c-4c0a-8d21-18f7d7350b68
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(162),
    oracle_id: "56719f6a-1a6c-4c0a-8d21-18f7d7350b68",
    scryfall_id: "b7387103-1df1-4fd0-9e91-1544509792c7",
    faces: &[FaceDef {
        name: "Swamp",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::BASIC,
        subtypes: &[subtypes::land::SWAMP],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
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
