//! Scarred Puma — {R} — Creature — Cat
//! Oracle: This creature can't attack unless a black or green creature also attacks.
//! Set: INV #163 — Invasion | Scryfall ID: 067ff95e-c4dc-41bb-9677-67f51a09b05a | Oracle ID: e28e1921-797b-4e6e-8058-3929af46eeb5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCARRED_PUMA,
    oracle_id = "e28e1921-797b-4e6e-8058-3929af46eeb5",
    scryfall_id = "067ff95e-c4dc-41bb-9677-67f51a09b05a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Scarred Puma",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CAT],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
