//! Elfhame Sanctuary — {1}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, you may search your library for a basic land card, reveal that card, put it into your hand, then shuffle. If you do, you skip your draw step this turn.
//! Set: INV #185 — Invasion | Scryfall ID: 6ab9a90c-5fd8-4f8c-b692-f98a2974810c | Oracle ID: 6f98f270-f6ff-4029-bcdd-3e7f6483eb5d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ELFHAME_SANCTUARY,
    oracle_id = "6f98f270-f6ff-4029-bcdd-3e7f6483eb5d",
    scryfall_id = "6ab9a90c-5fd8-4f8c-b692-f98a2974810c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Elfhame Sanctuary",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
