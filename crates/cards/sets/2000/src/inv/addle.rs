//! Addle — {1}{B} — Sorcery
//! Oracle: Choose a color. Target player reveals their hand and you choose a card of that color from it. That player discards that card.
//! Set: VMA #103 — Vintage Masters | Scryfall ID: f7fcce83-4378-405f-8007-dd0ed002be6b | Oracle ID: 3745aea4-4455-4859-9772-bcfd14e4067b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ADDLE,
    oracle_id = "3745aea4-4455-4859-9772-bcfd14e4067b",
    scryfall_id = "f7fcce83-4378-405f-8007-dd0ed002be6b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Addle",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
