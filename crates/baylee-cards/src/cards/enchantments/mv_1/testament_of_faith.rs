//! Testament of Faith — {W} — Enchantment
//! Oracle: {X}: This enchantment becomes an X/X Wall creature with defender in addition to its other types until end of turn.
//! Set: ODY #55 — Odyssey | Scryfall ID: f399b371-fed0-42b8-8f95-5d76fdbe193d | Oracle ID: bc9587cb-1b06-44ee-9bd4-c7b6c9394d45
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TESTAMENT_OF_FAITH,
    oracle_id = "bc9587cb-1b06-44ee-9bd4-c7b6c9394d45",
    scryfall_id = "f399b371-fed0-42b8-8f95-5d76fdbe193d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Testament of Faith",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
