//! Rhystic Tutor — {2}{B} — Sorcery
//! Oracle: Unless any player pays {2}, search your library for a card, put that card into your hand, then shuffle.
//! Set: PCY #77 — Prophecy | Scryfall ID: e02c1609-9cac-460f-8504-a84e28c340c1 | Oracle ID: 1c6da9a9-6395-460b-8150-ea46d7299dc2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RHYSTIC_TUTOR,
    oracle_id = "1c6da9a9-6395-460b-8150-ea46d7299dc2",
    scryfall_id = "e02c1609-9cac-460f-8504-a84e28c340c1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Rhystic Tutor",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
