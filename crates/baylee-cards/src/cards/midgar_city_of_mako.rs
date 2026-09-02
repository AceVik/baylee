//! Midgar, City of Mako // Reactor Raid — (no cost) — Land — Town // Sorcery — Adventure
//! Set: FIN #286 — Final Fantasy | Scryfall ID: 8a837256-6bb4-4a60-962d-d2793548d26c | Oracle ID: 4e34a49d-f031-48ac-a458-97b79124b76c
//! Face: Midgar, City of Mako —  — Land — Town
//! Face: Reactor Raid — {2}{B} — Sorcery — Adventure
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 755,
    oracle_id: "4e34a49d-f031-48ac-a458-97b79124b76c",
    scryfall_id: "8a837256-6bb4-4a60-962d-d2793548d26c",
    color_identity: ColorSet::from_slice(&[Color::Black]),
    faces: &[
    face! {
        name: "Midgar, City of Mako",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::TOWN],
    },
    face! {
        name: "Reactor Raid",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::SORCERY,
        subtypes: &[subtypes::spell::ADVENTURE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
