//! Crag Saurian — {R}{R}{R} — Creature — Lizard
//! Oracle: Whenever a source deals damage to this creature, that source's controller gains control of this creature.
//! Set: MMQ #185 — Mercadian Masques | Scryfall ID: 1f0907a5-938e-4ef4-aa85-e7c1ae4317a6 | Oracle ID: 7779e3be-5c3d-454f-88c1-9c0a97ebff09
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRAG_SAURIAN,
    oracle_id = "7779e3be-5c3d-454f-88c1-9c0a97ebff09",
    scryfall_id = "1f0907a5-938e-4ef4-aa85-e7c1ae4317a6",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Crag Saurian",
        mana_cost = mana!("{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::LIZARD],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
