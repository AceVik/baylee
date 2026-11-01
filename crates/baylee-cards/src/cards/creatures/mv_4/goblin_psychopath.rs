//! Goblin Psychopath — {3}{R} — Creature — Goblin Mutant
//! Oracle: Whenever this creature attacks or blocks, flip a coin. If you lose the flip, the next time it would deal combat damage this turn, it deals that damage to you instead.
//! Set: SCG #95 — Scourge | Scryfall ID: 52287036-00f1-4b6d-8cd8-b8cbc70c5135 | Oracle ID: 9369f131-3356-4926-9a9f-88a636305bd0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_PSYCHOPATH,
    oracle_id = "9369f131-3356-4926-9a9f-88a636305bd0",
    scryfall_id = "52287036-00f1-4b6d-8cd8-b8cbc70c5135",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Psychopath",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::MUTANT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
