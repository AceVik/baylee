//! Bad Moon — {1}{B} — Enchantment
//! Oracle: Black creatures get +1/+1.
//! Set: GVL #48 — Duel Decks Anthology: Garruk vs. Liliana | Scryfall ID: 8f8a75da-ea3c-43e7-9d32-1c92f8ec0fd2 | Oracle ID: fc5d3341-cbce-49e5-93cc-8add92479dca
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BAD_MOON,
    oracle_id = "fc5d3341-cbce-49e5-93cc-8add92479dca",
    scryfall_id = "8f8a75da-ea3c-43e7-9d32-1c92f8ec0fd2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Bad Moon",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
