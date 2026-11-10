//! Dawning Purist — {2}{W} — Creature — Human Cleric
//! Oracle: Whenever this creature deals combat damage to a player, you may destroy target enchantment that player controls.
//! Oracle: Morph {1}{W} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Set: ONS #22 — Onslaught | Scryfall ID: b8cb25b0-e4c3-4a4e-b722-ea30e695f917 | Oracle ID: 9e55cde0-16e6-4d32-8a37-1ef3dc812971
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DAWNING_PURIST,
    oracle_id = "9e55cde0-16e6-4d32-8a37-1ef3dc812971",
    scryfall_id = "b8cb25b0-e4c3-4a4e-b722-ea30e695f917",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Dawning Purist",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
