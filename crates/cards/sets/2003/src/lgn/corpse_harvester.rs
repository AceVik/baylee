//! Corpse Harvester — {3}{B}{B} — Creature — Zombie Wizard
//! Oracle: {1}{B}, {T}, Sacrifice a creature: Search your library for a Zombie card and a Swamp card, reveal them, put them into your hand, then shuffle.
//! Set: HOP #22 — Planechase | Scryfall ID: 39cc7fbb-010f-4e7c-b3bf-8853e54bc1b1 | Oracle ID: f02ad6fa-7a82-471a-979e-0bb9ba866ef6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CORPSE_HARVESTER,
    oracle_id = "f02ad6fa-7a82-471a-979e-0bb9ba866ef6",
    scryfall_id = "39cc7fbb-010f-4e7c-b3bf-8853e54bc1b1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Corpse Harvester",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::WIZARD],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
