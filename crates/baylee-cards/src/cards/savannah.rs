//! Savannah — (no cost) — Land — Forest Plains
//! Oracle: ({T}: Add {G} or {W}.)
//! Set: VMA #311 — Vintage Masters | Scryfall ID: b0d161fc-4a2a-4f1d-82b4-a746552552df | Oracle ID: 703243f0-8cb3-420f-958f-5fd4bde30293
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(138),
    oracle_id: "703243f0-8cb3-420f-958f-5fd4bde30293",
    scryfall_id: "b0d161fc-4a2a-4f1d-82b4-a746552552df",
    faces: &[FaceDef {
        name: "Savannah",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::FOREST, subtypes::land::PLAINS],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
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
