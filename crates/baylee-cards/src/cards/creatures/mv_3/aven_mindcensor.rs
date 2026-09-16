//! Aven Mindcensor — {2}{W} — Creature — Bird Wizard
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: If an opponent would search a library, that player searches the top four cards of that library instead.
//! Set: CLB #688 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: d4cf468f-4e9d-4551-a0ed-10bd6a2316ad | Oracle ID: d9517c5d-66d0-4178-96fb-a8c04f311ad8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AVEN_MINDCENSOR,
    oracle_id = "d9517c5d-66d0-4178-96fb-a8c04f311ad8",
    scryfall_id = "d4cf468f-4e9d-4551-a0ed-10bd6a2316ad",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Aven Mindcensor",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
