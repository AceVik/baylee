//! Saproling Infestation — {1}{G} — Enchantment
//! Oracle: Whenever a player kicks a spell, you create a 1/1 green Saproling creature token.
//! Set: INV #208 — Invasion | Scryfall ID: 8642e530-914c-4149-944a-c4966ee27299 | Oracle ID: dd257eb3-4112-41e5-9c7d-00d1f683dbdb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SAPROLING_INFESTATION,
    oracle_id = "dd257eb3-4112-41e5-9c7d-00d1f683dbdb",
    scryfall_id = "8642e530-914c-4149-944a-c4966ee27299",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Saproling Infestation",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
