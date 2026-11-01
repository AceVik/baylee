//! Denying Wind — {7}{U}{U} — Sorcery
//! Oracle: Search target player's library for up to seven cards and exile them. Then that player shuffles.
//! Set: PCY #32 — Prophecy | Scryfall ID: 15f236ce-41ad-4a49-a6f9-7853a2395a84 | Oracle ID: 8daed4d8-1a43-4452-85dd-51b320714489
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DENYING_WIND,
    oracle_id = "8daed4d8-1a43-4452-85dd-51b320714489",
    scryfall_id = "15f236ce-41ad-4a49-a6f9-7853a2395a84",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Denying Wind",
        mana_cost = mana!("{7}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
