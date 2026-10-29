//! Protective Sphere — {2}{W} — Enchantment
//! Oracle: {1}, Pay 1 life: Prevent all damage that would be dealt to you this turn by a source of your choice that shares a color with the mana spent on this activation cost. (Colorless mana prevents no damage.)
//! Set: INV #26 — Invasion | Scryfall ID: ef5ef13e-1cf0-42a9-95d0-30ade254d6a8 | Oracle ID: 2037c4bf-bf0d-46ab-8b4b-61d9a50c431e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PROTECTIVE_SPHERE,
    oracle_id = "2037c4bf-bf0d-46ab-8b4b-61d9a50c431e",
    scryfall_id = "ef5ef13e-1cf0-42a9-95d0-30ade254d6a8",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Protective Sphere",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
