//! Well-Laid Plans — {2}{U} — Enchantment
//! Oracle: Prevent all damage that would be dealt to a creature by another creature if they share a color.
//! Set: INV #88 — Invasion | Scryfall ID: 1c55eb8f-925a-42c1-9e48-d7f99cab3b01 | Oracle ID: 50343947-d127-4479-9c1b-8f5040a97afb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WELL_LAID_PLANS,
    oracle_id = "50343947-d127-4479-9c1b-8f5040a97afb",
    scryfall_id = "1c55eb8f-925a-42c1-9e48-d7f99cab3b01",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Well-Laid Plans",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
