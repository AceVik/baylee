//! Planeswalker's Fury — {2}{R} — Enchantment
//! Oracle: {3}{R}: Target opponent reveals a card at random from their hand. This enchantment deals damage equal to that card's mana value to that player. Activate only as a sorcery.
//! Set: PLS #70 — Planeshift | Scryfall ID: 6fa09e3a-bc7e-4292-aa5d-ce97c1b1f79f | Oracle ID: 2e482522-2eef-4f6c-887a-fcc51be61dc2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANESWALKER_S_FURY,
    oracle_id = "2e482522-2eef-4f6c-887a-fcc51be61dc2",
    scryfall_id = "6fa09e3a-bc7e-4292-aa5d-ce97c1b1f79f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Planeswalker's Fury",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
