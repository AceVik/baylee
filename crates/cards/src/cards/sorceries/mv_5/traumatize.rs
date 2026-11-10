//! Traumatize — {3}{U}{U} — Sorcery
//! Oracle: Target player mills half their library, rounded down.
//! Set: M14 #77 — Magic 2014 | Scryfall ID: 9b8784dd-83f9-41f8-aedc-f0f81073ffcb | Oracle ID: e2ea7d01-6564-4a7f-b935-49a5e3978dac
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TRAUMATIZE,
    oracle_id = "e2ea7d01-6564-4a7f-b935-49a5e3978dac",
    scryfall_id = "9b8784dd-83f9-41f8-aedc-f0f81073ffcb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Traumatize",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
