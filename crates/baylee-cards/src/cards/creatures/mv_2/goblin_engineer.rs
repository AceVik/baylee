//! Goblin Engineer — {1}{R} — Creature — Goblin Artificer
//! Oracle: When this creature enters, you may search your library for an artifact card, put it into your graveyard, then shuffle.
//! Oracle: {R}, {T}, Sacrifice an artifact: Return target artifact card with mana value 3 or less from your graveyard to the battlefield.
//! Set: MH1 #128 — Modern Horizons | Scryfall ID: a55c4d47-5252-40af-961d-c08bc688028a | Oracle ID: c1d6cce8-085f-42cb-8b0c-b6fbbf88b16a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GOBLIN_ENGINEER,
    oracle_id = "c1d6cce8-085f-42cb-8b0c-b6fbbf88b16a",
    scryfall_id = "a55c4d47-5252-40af-961d-c08bc688028a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Goblin Engineer",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::ARTIFICER],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
