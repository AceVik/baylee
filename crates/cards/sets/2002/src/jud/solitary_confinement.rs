//! Solitary Confinement — {2}{W} — Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you discard a card.
//! Oracle: Skip your draw step.
//! Oracle: You have shroud. (You can't be the target of spells or abilities.)
//! Oracle: Prevent all damage that would be dealt to you.
//! Set: MH2 #265 — Modern Horizons 2 | Scryfall ID: b9031a7f-8821-443c-9c9d-552fb25b2101 | Oracle ID: d41ab41d-07c8-4f6d-be5e-7aec9f5f6365
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOLITARY_CONFINEMENT,
    oracle_id = "d41ab41d-07c8-4f6d-be5e-7aec9f5f6365",
    scryfall_id = "b9031a7f-8821-443c-9c9d-552fb25b2101",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Solitary Confinement",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
