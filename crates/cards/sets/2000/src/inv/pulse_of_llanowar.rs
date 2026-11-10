//! Pulse of Llanowar — {3}{G} — Enchantment
//! Oracle: If a basic land you control is tapped for mana, it produces mana of a color of your choice instead of any other type.
//! Set: INV #202 — Invasion | Scryfall ID: db09afe5-5f01-4f77-a239-12d7a6e59024 | Oracle ID: dd1ef824-4e61-4dee-a8f0-9cf459008f2c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PULSE_OF_LLANOWAR,
    oracle_id = "dd1ef824-4e61-4dee-a8f0-9cf459008f2c",
    scryfall_id = "db09afe5-5f01-4f77-a239-12d7a6e59024",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Pulse of Llanowar",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
