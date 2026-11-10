//! Callous Giant — {4}{R}{R} — Creature — Giant
//! Oracle: If a source would deal 3 or less damage to this creature, prevent that damage.
//! Set: INV #139 — Invasion | Scryfall ID: 330028c4-8e91-4fe3-a87d-1660dfd2507e | Oracle ID: 7a2336ff-c8ed-4daa-b182-403c21bc8133
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CALLOUS_GIANT,
    oracle_id = "7a2336ff-c8ed-4daa-b182-403c21bc8133",
    scryfall_id = "330028c4-8e91-4fe3-a87d-1660dfd2507e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Callous Giant",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
