//! Extract — {U} — Sorcery
//! Oracle: Search target player's library for a card and exile it. Then that player shuffles.
//! Set: ODY #85 — Odyssey | Scryfall ID: 97f99a3d-d811-4666-aac8-5957068157dc | Oracle ID: 36e937a9-e15e-4404-bcdd-c2b0943a34ad
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EXTRACT,
    oracle_id = "36e937a9-e15e-4404-bcdd-c2b0943a34ad",
    scryfall_id = "97f99a3d-d811-4666-aac8-5957068157dc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Extract",
        mana_cost = mana!("{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
