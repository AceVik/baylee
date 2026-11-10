//! Daunting Defender — {4}{W} — Creature — Human Cleric
//! Oracle: If a source would deal damage to a Cleric creature you control, prevent 1 of that damage.
//! Set: ONS #21 — Onslaught | Scryfall ID: 38737f38-26bd-417c-b6b4-53f26e4e8044 | Oracle ID: 618f29e1-0ea0-4457-91a6-7210f7619c9a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DAUNTING_DEFENDER,
    oracle_id = "618f29e1-0ea0-4457-91a6-7210f7619c9a",
    scryfall_id = "38737f38-26bd-417c-b6b4-53f26e4e8044",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Daunting Defender",
        mana_cost = mana!("{4}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
