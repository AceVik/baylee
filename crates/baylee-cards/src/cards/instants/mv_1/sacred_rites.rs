//! Sacred Rites — {W} — Instant
//! Oracle: Discard any number of cards. Creatures you control get +0/+1 until end of turn for each card discarded this way.
//! Set: ODY #44 — Odyssey | Scryfall ID: 3d7f1dd0-ee1b-484a-949d-dde4c3340a09 | Oracle ID: 860a0e32-135b-4972-a1af-ebac60cae08f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SACRED_RITES,
    oracle_id = "860a0e32-135b-4972-a1af-ebac60cae08f",
    scryfall_id = "3d7f1dd0-ee1b-484a-949d-dde4c3340a09",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sacred Rites",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
