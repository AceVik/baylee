//! Twists and Turns // Mycoid Maze — {G} — Enchantment // Land — Cave
//! Oracle: If a creature you control would explore, instead you scry 1, then that creature explores.
//! Oracle: When this enchantment enters, target creature you control explores.
//! Oracle: When a land you control enters, if you control seven or more lands, transform this enchantment.
//! Oracle: (Transforms from Twists and Turns.)
//! Oracle: {T}: Add {G}.
//! Oracle: {3}{G}, {T}: Look at the top four cards of your library. You may reveal a creature card from among them and put that card into your hand. Put the rest on the bottom of your library in a random order.
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
