//! Divine Presence — {2}{W} — Enchantment
//! Oracle: If a source would deal 4 or more damage to a permanent or player, that source deals 3 damage to that permanent or player instead.
//! Set: INV #15 — Invasion | Scryfall ID: 28cb898d-d6ce-410a-83bf-37962cca2735 | Oracle ID: ff9a5529-83bd-46a8-b20b-235a5fd26d93
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIVINE_PRESENCE,
    oracle_id = "ff9a5529-83bd-46a8-b20b-235a5fd26d93",
    scryfall_id = "28cb898d-d6ce-410a-83bf-37962cca2735",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Divine Presence",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
