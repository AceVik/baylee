//! Saprazzan Bailiff — {3}{U}{U} — Creature — Merfolk
//! Oracle: When this creature enters, exile all artifact and enchantment cards from all graveyards.
//! Oracle: When this creature leaves the battlefield, return all artifact and enchantment cards from all graveyards to their owners' hands.
//! Set: MMQ #97 — Mercadian Masques | Scryfall ID: a9f50964-1b57-426a-ac46-c90c045c7e40 | Oracle ID: 1f861517-ac53-4218-b62e-35e971098b6d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SAPRAZZAN_BAILIFF,
    oracle_id = "1f861517-ac53-4218-b62e-35e971098b6d",
    scryfall_id = "a9f50964-1b57-426a-ac46-c90c045c7e40",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Saprazzan Bailiff",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
