//! Raugrin Triome — (no cost) — Land — Island Mountain Plains
//! Oracle: ({T}: Add {U}, {R}, or {W}.)
//! Oracle: This land enters tapped.
//! Oracle: Cycling {3} ({3}, Discard this card: Draw a card.)
//! Set: IKO #251 — Ikoria: Lair of Behemoths | Scryfall ID: 02138fbb-3962-4348-8d31-faaefba0b8b2 | Oracle ID: c7fa1dda-9312-4ec8-82cd-a1ba7bc33497
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(123),
    oracle_id: "c7fa1dda-9312-4ec8-82cd-a1ba7bc33497",
    scryfall_id: "02138fbb-3962-4348-8d31-faaefba0b8b2",
    faces: &[FaceDef {
        name: "Raugrin Triome",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[
            subtypes::land::ISLAND,
            subtypes::land::MOUNTAIN,
            subtypes::land::PLAINS,
        ],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Red, Color::Blue, Color::White]),
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
