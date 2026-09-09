//! Jidoor, Aristocratic Capital // Overture — (no cost) — Land — Town // Sorcery — Adventure
//! Set: FIN #284 — Final Fantasy | Scryfall ID: 98b2d5b5-f85b-4c42-a0f5-a76f6af304ba | Oracle ID: bd513d9d-5aa2-4860-bd86-8b5d9430f133
//! Face: Jidoor, Aristocratic Capital —  — Land — Town
//! Face: Overture — {4}{U}{U} — Sorcery — Adventure
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 667,
    oracle_id: "bd513d9d-5aa2-4860-bd86-8b5d9430f133",
    scryfall_id: "98b2d5b5-f85b-4c42-a0f5-a76f6af304ba",
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    faces: &[
    face! {
        name: "Jidoor, Aristocratic Capital",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::TOWN],
    },
    face! {
        name: "Overture",
        mana_cost: baylee_core::mana!("{4}{U}{U}"),
        types: TypeSet::SORCERY,
        subtypes: &[subtypes::spell::ADVENTURE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
