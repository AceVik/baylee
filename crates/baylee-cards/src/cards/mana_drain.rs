//! Mana Drain — {U}{U} — Instant
//! Oracle: Counter target spell. At the beginning of your next main phase, add an amount of {C} equal to that spell's mana value.
//! Set: 2X2 #57 — Double Masters 2022 | Scryfall ID: 3c429c40-2389-41e5-8681-4bb274e25eba | Oracle ID: 74d3277a-38e5-4732-afed-084a56148f20
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(90),
    oracle_id: "74d3277a-38e5-4732-afed-084a56148f20",
    scryfall_id: "3c429c40-2389-41e5-8681-4bb274e25eba",
    faces: &[FaceDef {
        name: "Mana Drain",
        mana_cost: baylee_core::mana!("{U}{U}"),
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
