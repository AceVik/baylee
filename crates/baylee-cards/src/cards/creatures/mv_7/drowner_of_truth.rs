//! Drowner of Truth // Drowned Jungle — {5}{G/U}{G/U} — Creature — Eldrazi // Land
//! Set: MH3 #253 — Modern Horizons 3 | Scryfall ID: 7a1d3c1d-1373-4ac4-bb26-9780976efc4f | Oracle ID: db19a27a-ee22-4931-ae3c-0ce21f456ea6
//! Face: Drowner of Truth — {5}{G/U}{G/U} — Creature — Eldrazi
//! Face: Drowned Jungle —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 444,
    oracle_id: "db19a27a-ee22-4931-ae3c-0ce21f456ea6",
    scryfall_id: "7a1d3c1d-1373-4ac4-bb26-9780976efc4f",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces: &[
    face! {
        name: "Drowner of Truth",
        mana_cost: baylee_core::mana!("{5}{G/U}{G/U}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::ELDRAZI],
        power: Some(7),
        toughness: Some(6),
    },
    face! {
        name: "Drowned Jungle",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
