//! Twists and Turns // Mycoid Maze — {G} — Enchantment // Land — Cave
//! Set: LCI #217 — The Lost Caverns of Ixalan | Scryfall ID: 3cdf691e-96a5-45c7-9b94-6f04af81c8e4 | Oracle ID: 740aa9d9-91a9-431e-8bf9-1344e5273e27
//! Face: Twists and Turns — {G} — Enchantment
//! Face: Mycoid Maze —  — Land — Cave
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1242,
    oracle_id: "740aa9d9-91a9-431e-8bf9-1344e5273e27",
    scryfall_id: "3cdf691e-96a5-45c7-9b94-6f04af81c8e4",
    color_identity: ColorSet::from_slice(&[Color::Green]),
    faces: &[
    face! {
        name: "Twists and Turns",
        mana_cost: baylee_core::mana!("{G}"),
        types: TypeSet::ENCHANTMENT,
    },
    face! {
        name: "Mycoid Maze",
        types: TypeSet::LAND,
        subtypes: &[subtypes::land::CAVE],
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
