//! Judgment Bolt — {3}{R} — Instant
//! Oracle: Judgment Bolt deals 5 damage to target creature and X damage to that creature's controller, where X is the number of Equipment you control.
//! Set: FIN #559 — Final Fantasy | Scryfall ID: 05b04b06-9271-4a28-a60e-287df0d1a4d1 | Oracle ID: aad9f9f2-4c6b-48d0-80bd-77b73a9ea5f1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JUDGMENT_BOLT,
    oracle_id = "aad9f9f2-4c6b-48d0-80bd-77b73a9ea5f1",
    scryfall_id = "05b04b06-9271-4a28-a60e-287df0d1a4d1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Judgment Bolt",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
