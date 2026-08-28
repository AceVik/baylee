//! Swords to Plowshares — {W} — Instant
//! Oracle: Exile target creature. Its controller gains life equal to its power.
//! Set: MSC #143 — Marvel Super Heroes Commander | Scryfall ID: b4e9c870-23c0-413a-ae39-265f09da16d1 | Oracle ID: b1544f21-7e98-461b-aed5-e748b0168c52
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(164),
    oracle_id: "b1544f21-7e98-461b-aed5-e748b0168c52",
    scryfall_id: "b4e9c870-23c0-413a-ae39-265f09da16d1",
    faces: &[FaceDef {
        name: "Swords to Plowshares",
        mana_cost: baylee_core::mana!("{W}"),
        types: TypeSet::INSTANT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
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
