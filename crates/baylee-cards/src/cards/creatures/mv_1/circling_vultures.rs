//! Circling Vultures — {B} — Creature — Bird
//! Oracle: Flying
//! Oracle: You may discard this card any time you could cast an instant.
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you exile the top creature card of your graveyard.
//! Set: WTH #64 — Weatherlight | Scryfall ID: 8dae8e49-c2b6-4965-9249-49f93449d271 | Oracle ID: 7fd6fe21-8fb8-499b-9dd4-806d0d660265
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CIRCLING_VULTURES,
    oracle_id = "7fd6fe21-8fb8-499b-9dd4-806d0d660265",
    scryfall_id = "8dae8e49-c2b6-4965-9249-49f93449d271",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Circling Vultures",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
