//! Skirk Fire Marshal — {3}{R}{R} — Creature — Goblin
//! Oracle: Protection from red
//! Oracle: Tap five untapped Goblins you control: This creature deals 10 damage to each creature and each player.
//! Set: EVG #50 — Duel Decks Anthology: Elves vs. Goblins | Scryfall ID: aac5a2ff-c9fc-4ae1-8207-69d7fe4fd11f | Oracle ID: 87105cdf-c28e-4fd9-83d0-0fef1704b174
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKIRK_FIRE_MARSHAL,
    oracle_id = "87105cdf-c28e-4fd9-83d0-0fef1704b174",
    scryfall_id = "aac5a2ff-c9fc-4ae1-8207-69d7fe4fd11f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Skirk Fire Marshal",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
