//! Shoreline Ranger — {5}{U} — Creature — Bird Soldier Ranger
//! Oracle: Flying
//! Oracle: Islandcycling {2} ({2}, Discard this card: Search your library for an Island card, reveal it, put it into your hand, then shuffle.)
//! Set: A25 #72 — Masters 25 | Scryfall ID: a9957cbe-da88-41fe-a4aa-e32ec9e7aa38 | Oracle ID: 606fbf99-d3e3-4131-b991-3cb71c003834
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SHORELINE_RANGER,
    oracle_id = "606fbf99-d3e3-4131-b991-3cb71c003834",
    scryfall_id = "a9957cbe-da88-41fe-a4aa-e32ec9e7aa38",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Shoreline Ranger",
        mana_cost = mana!("{5}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::BIRD,
            subtypes::creature::SOLDIER,
            subtypes::creature::RANGER
        ],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
