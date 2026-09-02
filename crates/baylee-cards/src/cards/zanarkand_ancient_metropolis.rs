//! Zanarkand, Ancient Metropolis // Lasting Fayth — (no cost) — Land — Town // Sorcery — Adventure
//! Set: FIN #293 — Final Fantasy | Scryfall ID: 881e4c00-3b9a-47a1-bf66-1badda994c88 | Oracle ID: 5f2b3ea8-99ee-47a4-8a1c-4b27478d524c
//! Face: Zanarkand, Ancient Metropolis —  — Land — Town
//! Face: Lasting Fayth — {4}{G}{G} — Sorcery — Adventure
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1339,
    oracle_id: "5f2b3ea8-99ee-47a4-8a1c-4b27478d524c",
    scryfall_id: "881e4c00-3b9a-47a1-bf66-1badda994c88",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Zanarkand, Ancient Metropolis",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::TOWN],
    },
    face! {
        name: "Lasting Fayth",
        mana_cost: baylee_core::mana!("{4}{G}{G}"),
        types: TypeSet::SORCERY,
        subtypes: &[subtypes::spell::ADVENTURE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
