//! Upwelling — {3}{G} — Enchantment
//! Oracle: Players don't lose unspent mana as steps and phases end.
//! Set: 10E #306 — Tenth Edition | Scryfall ID: d5c47cac-679f-4b48-977f-2fa149a6c3c3 | Oracle ID: 420ff0f4-6056-4b40-9a28-fd4c23d6b81c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UPWELLING,
    oracle_id = "420ff0f4-6056-4b40-9a28-fd4c23d6b81c",
    scryfall_id = "d5c47cac-679f-4b48-977f-2fa149a6c3c3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Upwelling",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
