//! Mystical Tutor — {U} — Instant
//! Oracle: Search your library for an instant or sorcery card, reveal it, then shuffle and put that card on top.
//! Set: DMR #60 — Dominaria Remastered | Scryfall ID: 36fa9a0b-b0c9-43ea-ba11-99d7982f974e | Oracle ID: fb81f95c-70f8-4eb7-8d15-15d0ae23ec03
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(102),
    oracle_id: "fb81f95c-70f8-4eb7-8d15-15d0ae23ec03",
    scryfall_id: "36fa9a0b-b0c9-43ea-ba11-99d7982f974e",
    faces: &[FaceDef {
        name: "Mystical Tutor",
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
