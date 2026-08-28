//! Brainstorm — {U} — Instant
//! Oracle: Draw three cards, then put two cards from your hand on top of your library in any order.
//! Set: TLE #155 — Avatar: The Last Airbender Eternal | Scryfall ID: b5545882-6963-4729-b2c6-fb4bdc75ffcc | Oracle ID: 36cd2364-d113-47d1-b2c4-b088d9eb88dd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(15),
    oracle_id: "36cd2364-d113-47d1-b2c4-b088d9eb88dd",
    scryfall_id: "b5545882-6963-4729-b2c6-fb4bdc75ffcc",
    faces: &[FaceDef {
        name: "Brainstorm",
        mana_cost: baylee_core::mana!("{U}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
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
