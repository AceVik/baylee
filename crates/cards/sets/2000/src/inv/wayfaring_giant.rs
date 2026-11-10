//! Wayfaring Giant — {5}{W} — Creature — Giant
//! Oracle: Domain — This creature gets +1/+1 for each basic land type among lands you control.
//! Set: INV #44 — Invasion | Scryfall ID: 57e45de5-0e8b-41d3-979b-ec5a29cac682 | Oracle ID: e20e5ad9-36b5-4962-acc2-555d069c5daf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WAYFARING_GIANT,
    oracle_id = "e20e5ad9-36b5-4962-acc2-555d069c5daf",
    scryfall_id = "57e45de5-0e8b-41d3-979b-ec5a29cac682",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Wayfaring Giant",
        mana_cost = mana!("{5}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GIANT],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
