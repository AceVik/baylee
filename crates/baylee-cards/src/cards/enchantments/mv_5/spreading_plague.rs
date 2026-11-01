//! Spreading Plague — {4}{B} — Enchantment
//! Oracle: Whenever a creature enters, destroy all other creatures that share a color with it. They can't be regenerated.
//! Set: INV #125 — Invasion | Scryfall ID: ac86055d-ce08-4b05-a92c-45e007ca0ba4 | Oracle ID: 6c134ef7-5c2f-4e4c-ae40-ef338fbf41e6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPREADING_PLAGUE,
    oracle_id = "6c134ef7-5c2f-4e4c-ae40-ef338fbf41e6",
    scryfall_id = "ac86055d-ce08-4b05-a92c-45e007ca0ba4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Spreading Plague",
        mana_cost = mana!("{4}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
