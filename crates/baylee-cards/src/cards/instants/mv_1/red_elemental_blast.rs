//! Red Elemental Blast — {R} — Instant
//! Oracle: Choose one —
//! Oracle: • Counter target blue spell.
//! Oracle: • Destroy target blue permanent.
//! Set: A25 #147 — Masters 25 | Scryfall ID: 70a45e9b-699e-425a-9f3d-267274830d3e | Oracle ID: bb329a5c-b9f9-4973-a53f-090024146325
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RED_ELEMENTAL_BLAST,
    oracle_id = "bb329a5c-b9f9-4973-a53f-090024146325",
    scryfall_id = "70a45e9b-699e-425a-9f3d-267274830d3e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Red Elemental Blast",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
