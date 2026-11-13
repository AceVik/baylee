//! Acceptable Losses — {3}{R} — Sorcery
//! Oracle: As an additional cost to cast this spell, discard a card at random.
//! Oracle: Acceptable Losses deals 5 damage to target creature.
//! Set: ODY #172 — Odyssey | Scryfall ID: 9082bfb8-0ab0-4378-976d-a2c9d3c35a5e | Oracle ID: 2b6d95b5-0680-43ae-9ff3-8e287a9eb837
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ACCEPTABLE_LOSSES,
    oracle_id = "2b6d95b5-0680-43ae-9ff3-8e287a9eb837",
    scryfall_id = "9082bfb8-0ab0-4378-976d-a2c9d3c35a5e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Acceptable Losses",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
