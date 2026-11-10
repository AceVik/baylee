//! Goblin Assassin — {3}{R}{R} — Creature — Goblin Assassin
//! Oracle: Whenever this creature or another Goblin enters, each player flips a coin. Each player whose coin comes up tails sacrifices a creature of their choice.
//! Set: LGN #95 — Legions | Scryfall ID: 57ec836f-6dcf-45f9-8e95-487762742a1e | Oracle ID: faf9bb36-e537-42a8-b9ee-bf2c50b0e5bd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_ASSASSIN,
    oracle_id = "faf9bb36-e537-42a8-b9ee-bf2c50b0e5bd",
    scryfall_id = "57ec836f-6dcf-45f9-8e95-487762742a1e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Assassin",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::ASSASSIN],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
