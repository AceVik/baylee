//! Browse — {2}{U}{U} — Enchantment
//! Oracle: {2}{U}{U}: Look at the top five cards of your library, put one of them into your hand, and exile the rest.
//! Set: ME2 #43 — Masters Edition II | Scryfall ID: 6c3dc360-4f7b-49bc-801a-89eb06fbb729 | Oracle ID: 49491601-e272-4603-9348-185a2953bd4b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BROWSE,
    oracle_id = "49491601-e272-4603-9348-185a2953bd4b",
    scryfall_id = "6c3dc360-4f7b-49bc-801a-89eb06fbb729",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Browse",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
