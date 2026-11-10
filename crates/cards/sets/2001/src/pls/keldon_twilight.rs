//! Keldon Twilight — {1}{B}{R} — Enchantment
//! Oracle: At the beginning of each player's end step, if no creatures attacked this turn, that player sacrifices a creature of their choice that they controlled since the beginning of the turn.
//! Set: PLS #112 — Planeshift | Scryfall ID: e071665e-bb72-42e0-a42d-0d0ff02abd2b | Oracle ID: c886a076-dbe2-4d80-8dab-537b84d0837a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KELDON_TWILIGHT,
    oracle_id = "c886a076-dbe2-4d80-8dab-537b84d0837a",
    scryfall_id = "e071665e-bb72-42e0-a42d-0d0ff02abd2b",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Keldon Twilight",
        mana_cost = mana!("{1}{B}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
