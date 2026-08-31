//! Mystical Tutor — {U} — Instant
//! Oracle: Search your library for an instant or sorcery card, reveal it, then shuffle and put that card on top.
//! Set: DMR #60 — Dominaria Remastered | Scryfall ID: 36fa9a0b-b0c9-43ea-ba11-99d7982f974e | Oracle ID: fb81f95c-70f8-4eb7-8d15-15d0ae23ec03
// IMPLEMENTED — filtered tutor to the top of the library (reveal is M3).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    SearchDest,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static FIND: Filter = Filter::Or(&[
    Filter::HasType(TypeSet::INSTANT),
    Filter::HasType(TypeSet::SORCERY),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(102),
    oracle_id: "fb81f95c-70f8-4eb7-8d15-15d0ae23ec03",
    scryfall_id: "36fa9a0b-b0c9-43ea-ba11-99d7982f974e",
    faces: &[FaceDef {
        name: "Mystical Tutor",
        mana_cost: baylee_core::mana!("{U}"),
        types: TypeSet::INSTANT,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::SearchLibrary {
            filter: &FIND,
            dest: SearchDest::TopOfLibrary,
            tapped: false,
            shuffle: true,
            optional: false,
        }],
        targets: None,
    }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
