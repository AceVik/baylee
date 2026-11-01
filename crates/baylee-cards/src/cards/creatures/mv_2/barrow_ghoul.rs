//! Barrow Ghoul — {1}{B} — Creature — Zombie
//! Oracle: At the beginning of your upkeep, sacrifice this creature unless you exile the top creature card of your graveyard.
//! Set: WTH #61 — Weatherlight | Scryfall ID: f7055007-83dd-40fe-b2a1-4b3132f636db | Oracle ID: 1610875f-48da-490c-a371-c439dc36f533
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BARROW_GHOUL,
    oracle_id = "1610875f-48da-490c-a371-c439dc36f533",
    scryfall_id = "f7055007-83dd-40fe-b2a1-4b3132f636db",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Barrow Ghoul",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
