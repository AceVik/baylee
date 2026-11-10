//! Pollen Remedy — {W} — Instant
//! Oracle: Kicker—Sacrifice a land. (You may sacrifice a land in addition to any other costs as you cast this spell.)
//! Oracle: Prevent the next 3 damage that would be dealt this turn to any number of targets, divided as you choose. If this spell was kicked, prevent the next 6 damage this way instead.
//! Set: PLS #13 — Planeshift | Scryfall ID: 9797c813-0cda-44ad-ae41-330e9bde9cb9 | Oracle ID: ff52f955-98de-4f12-9ec7-f1d73e3edae9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::POLLEN_REMEDY,
    oracle_id = "ff52f955-98de-4f12-9ec7-f1d73e3edae9",
    scryfall_id = "9797c813-0cda-44ad-ae41-330e9bde9cb9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Pollen Remedy",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
