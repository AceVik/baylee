//! Cultivate — {2}{G} — Sorcery
//! Oracle: Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle.
//! Set: MSC #172 — Marvel Super Heroes Commander | Scryfall ID: e60deb92-f7dd-4f4e-9036-e47dd586f985 | Oracle ID: 8b755881-a72d-4e21-a369-d2924eb4585a
// IMPLEMENTED — two finds with different destinations, which is exactly what
// `SearchLibrary` could not say before it carried a `finds` slice.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, Find, KeywordSet,
    PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

/// A basic land card: the *supertype* Basic plus the land type (CR 205.4a).
static BASIC_LAND: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::HasType(TypeSet::LAND),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(194),
    oracle_id: "8b755881-a72d-4e21-a369-d2924eb4585a",
    scryfall_id: "e60deb92-f7dd-4f4e-9036-e47dd586f985",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[FaceDef {
        name: "Cultivate",
        mana_cost: baylee_core::mana!("{2}{G}"),
        types: TypeSet::SORCERY,
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        // "put one onto the battlefield tapped and the other into your hand"
        // — the order here is the order the text names them, and it is the
        // order a single find falls back to.
        effects: &[Effect::SearchLibrary {
            filter: &BASIC_LAND,
            finds: &[Find::BATTLEFIELD_TAPPED, Find::HAND],
            shuffle: true,
            optional: true, // "up to two"
        }],
        targets: None,
    }],
    ..CardDef::DEFAULT
};

// Engine-level coverage lives in baylee-engine (search_tests): two basic
// lands are found, the first enters tapped and the second lands in hand.
