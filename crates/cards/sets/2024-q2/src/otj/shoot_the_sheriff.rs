//! Shoot the Sheriff — {1}{B} — Instant
//! Oracle: Destroy target non-outlaw creature. (Assassins, Mercenaries, Pirates, Rogues, and Warlocks are outlaws. Everyone else is fair game.)
//! Set: OTJ #106 — Outlaws of Thunder Junction | Scryfall ID: 180d6528-c524-4bb8-8a72-b3775cd2c177 | Oracle ID: 2038138e-2e52-41e4-90fc-443fa054c5c0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHOOT_THE_SHERIFF,
    oracle_id = "2038138e-2e52-41e4-90fc-443fa054c5c0",
    scryfall_id = "180d6528-c524-4bb8-8a72-b3775cd2c177",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Shoot the Sheriff",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
