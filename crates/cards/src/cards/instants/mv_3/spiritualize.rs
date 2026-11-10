//! Spiritualize — {2}{W} — Instant
//! Oracle: Until end of turn, whenever target creature deals damage, you gain that much life.
//! Oracle: Draw a card.
//! Set: ODY #53 — Odyssey | Scryfall ID: f6780ded-86df-419d-b63b-6b67d2960b28 | Oracle ID: 15c4a13e-33ae-4d08-bb1a-535d8b82ca07
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPIRITUALIZE,
    oracle_id = "15c4a13e-33ae-4d08-bb1a-535d8b82ca07",
    scryfall_id = "f6780ded-86df-419d-b63b-6b67d2960b28",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Spiritualize",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
