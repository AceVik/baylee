//! Kodama's Reach — {2}{G} — Sorcery — Arcane
//! Oracle: Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle.
//! Set: ECC #113 — Lorwyn Eclipsed Commander | Scryfall ID: 90c423cc-1264-4067-9c50-e7c88c68ef2d | Oracle ID: 1593ea18-2f2f-4ab4-83fb-6ccc0bec8a90
// IMPLEMENTED — the same two finds as Cultivate. Arcane is a spell subtype
// carrying no rules of its own; splice onto Arcane is not modelled and no
// card here has it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// A basic land card: the *supertype* Basic plus the land type (CR 205.4a).
static BASIC_LAND: Filter = Filter::And(&[Filter::HasSupertype(SupertypeSet::BASIC), Filter::LAND]);

card! {
    index: 195,
    oracle_id: "1593ea18-2f2f-4ab4-83fb-6ccc0bec8a90",
    scryfall_id: "90c423cc-1264-4067-9c50-e7c88c68ef2d",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[face! {
        name: "Kodama's Reach",
        mana_cost: baylee_core::mana!("{2}{G}"),
        types: TypeSet::SORCERY,
        subtypes: &[subtypes::spell::ARCANE],
    }],
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::SearchLibrary {
            filter: &BASIC_LAND,
            finds: &[Find::BATTLEFIELD_TAPPED, Find::HAND],
            optional: true, // "up to two"
        }])],
}

// Engine-level coverage lives in baylee-engine (search_tests), on Cultivate:
// the same effect with the same two destinations.
