//! Raka Sanctuary — {2}{R} — Enchantment
//! Oracle: At the beginning of your upkeep, if you control a white or blue permanent, this enchantment deals 1 damage to target creature. If you control a white permanent and a blue permanent, this enchantment deals 3 damage instead.
//! Set: APC #67 — Apocalypse | Scryfall ID: 62cab0be-589c-42a0-a297-1faaec46c73f | Oracle ID: 0b085c90-95df-4823-ba67-9eb398b1ec94
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAKA_SANCTUARY,
    oracle_id = "0b085c90-95df-4823-ba67-9eb398b1ec94",
    scryfall_id = "62cab0be-589c-42a0-a297-1faaec46c73f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Raka Sanctuary",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
