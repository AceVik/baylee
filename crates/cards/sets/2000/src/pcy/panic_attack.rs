//! Panic Attack — {2}{R} — Sorcery
//! Oracle: Up to three target creatures can't block this turn.
//! Set: M10 #150 — Magic 2010 | Scryfall ID: e9c6cbc8-e33a-4692-aff0-4de9bb65421d | Oracle ID: c1346677-d89e-49d0-9819-08805f00bb64
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PANIC_ATTACK,
    oracle_id = "c1346677-d89e-49d0-9819-08805f00bb64",
    scryfall_id = "e9c6cbc8-e33a-4692-aff0-4de9bb65421d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Panic Attack",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
