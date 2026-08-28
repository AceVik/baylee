//! Prairie Stream — (no cost) — Land — Plains Island
//! Oracle: ({T}: Add {W} or {U}.)
//! Oracle: This land enters tapped unless you control two or more basic lands.
//! Set: MSC #257 — Marvel Super Heroes Commander | Scryfall ID: b2e133b4-2263-4ac2-8d16-7bf307d5e104 | Oracle ID: 5330e24a-8568-446e-840a-594cd08bd1bc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(117),
    oracle_id: "5330e24a-8568-446e-840a-594cd08bd1bc",
    scryfall_id: "b2e133b4-2263-4ac2-8d16-7bf307d5e104",
    faces: &[FaceDef {
        name: "Prairie Stream",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[subtypes::land::PLAINS, subtypes::land::ISLAND],
        power: None,
        toughness: None,
        loyalty: None,
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
