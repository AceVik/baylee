//! Stone Giant — {2}{R}{R} — Creature — Giant
//! Oracle: {T}: Target creature you control with toughness less than this creature's power gains flying until end of turn. Destroy that creature at the beginning of the next end step.
//! Set: DDI #55 — Duel Decks: Venser vs. Koth | Scryfall ID: fbd9a10e-2bf7-4af7-bb34-52de01e03523 | Oracle ID: 0b8e3f9b-a4da-49a3-8545-ce7a265e5856
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STONE_GIANT,
    oracle_id = "0b8e3f9b-a4da-49a3-8545-ce7a265e5856",
    scryfall_id = "fbd9a10e-2bf7-4af7-bb34-52de01e03523",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Stone Giant",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(3),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
