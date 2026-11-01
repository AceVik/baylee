//! Goblin Spy — {R} — Creature — Goblin Rogue
//! Oracle: Play with the top card of your library revealed.
//! Set: INV #145 — Invasion | Scryfall ID: 2a89a099-8805-4b26-babd-5d9f48ee406a | Oracle ID: 5f1e8aa2-6b85-4e2e-92e4-876470447d6f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_SPY,
    oracle_id = "5f1e8aa2-6b85-4e2e-92e4-876470447d6f",
    scryfall_id = "2a89a099-8805-4b26-babd-5d9f48ee406a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Spy",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::ROGUE],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
