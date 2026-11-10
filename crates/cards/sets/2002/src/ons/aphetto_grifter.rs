//! Aphetto Grifter — {2}{U} — Creature — Human Wizard
//! Oracle: Tap two untapped Wizards you control: Tap target permanent.
//! Set: ONS #65 — Onslaught | Scryfall ID: 3a7a7bf3-1b0c-415d-9c57-73ac55b1f915 | Oracle ID: d9d7b7a9-5107-4720-9f92-b49e372372b8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::APHETTO_GRIFTER,
    oracle_id = "d9d7b7a9-5107-4720-9f92-b49e372372b8",
    scryfall_id = "3a7a7bf3-1b0c-415d-9c57-73ac55b1f915",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Aphetto Grifter",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
