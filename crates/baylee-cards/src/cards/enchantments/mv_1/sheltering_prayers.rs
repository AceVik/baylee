//! Sheltering Prayers — {W} — Enchantment
//! Oracle: Basic lands each player controls have shroud as long as that player controls three or fewer lands. (They can't be the targets of spells or abilities.)
//! Set: PCY #22 — Prophecy | Scryfall ID: a30803e6-7f0e-4832-b121-b18480c6465c | Oracle ID: 44075eea-ab9d-4252-b77c-597d19a10fc4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHELTERING_PRAYERS,
    oracle_id = "44075eea-ab9d-4252-b77c-597d19a10fc4",
    scryfall_id = "a30803e6-7f0e-4832-b121-b18480c6465c",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Sheltering Prayers",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
