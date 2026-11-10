//! Tombfire — {B} — Sorcery
//! Oracle: Target player exiles all cards with flashback from their graveyard.
//! Set: ODY #165 — Odyssey | Scryfall ID: bb08a4a7-941b-4b54-a89b-a4145a6fc14e | Oracle ID: 37feb05b-1f69-4d0d-837c-592461a431e0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOMBFIRE,
    oracle_id = "37feb05b-1f69-4d0d-837c-592461a431e0",
    scryfall_id = "bb08a4a7-941b-4b54-a89b-a4145a6fc14e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Tombfire",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
