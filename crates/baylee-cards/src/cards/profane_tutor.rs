//! Profane Tutor — (no cost) — Sorcery
//! Oracle: Suspend 2—{1}{B} (Rather than cast this card from your hand, pay {1}{B} and exile it with two time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)
//! Oracle: Search your library for a card, put that card into your hand, then shuffle.
//! Set: MH2 #97 — Modern Horizons 2 | Scryfall ID: 2afc6f7d-ab59-4d64-bd11-6bd0fd4bfcd2 | Oracle ID: 27a1f42c-0b86-4609-9609-1fa9cab7e7c9
// IMPLEMENTED — suspend 2 demonic-tutor variant.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    SearchDest,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_CARD: Filter = Filter::Any;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(120),
    oracle_id: "27a1f42c-0b86-4609-9609-1fa9cab7e7c9",
    scryfall_id: "2afc6f7d-ab59-4d64-bd11-6bd0fd4bfcd2",
    faces: &[FaceDef {
        name: "Profane Tutor",
        types: TypeSet::SORCERY,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Suspend {
            counters: 2,
            cost: baylee_core::mana!("{1}{B}"),
        },
        AbilityDef::Spell {
            effects: &[Effect::SearchLibrary {
                filter: &ANY_CARD,
                dest: SearchDest::Hand,
                tapped: false,
                shuffle: true,
                optional: false,
            }],
            targets: None,
        },
    ],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
