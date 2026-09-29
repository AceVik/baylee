//! Stasis — {1}{U} — Enchantment
//! Oracle: Players skip their untap steps.
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay {U}.
//! Set: ME4 #64 — Masters Edition IV | Scryfall ID: 62f99124-6595-45f8-bece-1775e4c55a5c | Oracle ID: a8cf1379-0195-4e11-b994-481ef1284245
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STASIS,
    oracle_id = "a8cf1379-0195-4e11-b994-481ef1284245",
    scryfall_id = "62f99124-6595-45f8-bece-1775e4c55a5c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Stasis",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
