//! Scavenging Ghoul — {3}{B} — Creature — Zombie
//! Oracle: At the beginning of each end step, put a corpse counter on this creature for each creature that died this turn.
//! Oracle: Remove a corpse counter from this creature: Regenerate this creature.
//! Set: ME4 #95 — Masters Edition IV | Scryfall ID: 78d5eb2e-ca20-4c28-a995-c69c92fc1024 | Oracle ID: 68c0c04e-b0d5-4721-83bf-bf18e8b7e680
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCAVENGING_GHOUL,
    oracle_id = "68c0c04e-b0d5-4721-83bf-bf18e8b7e680",
    scryfall_id = "78d5eb2e-ca20-4c28-a995-c69c92fc1024",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Scavenging Ghoul",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
