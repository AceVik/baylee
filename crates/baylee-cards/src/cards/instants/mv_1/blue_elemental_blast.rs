//! Blue Elemental Blast — {U} — Instant
//! Oracle: Choose one —
//! Oracle: • Counter target red spell.
//! Oracle: • Destroy target red permanent.
//! Set: A25 #43 — Masters 25 | Scryfall ID: 2f51f88f-f662-4572-a371-9a77718ed079 | Oracle ID: 65e1558c-6b09-4ddc-b520-f19f4fb972af
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BLUE_ELEMENTAL_BLAST,
    oracle_id = "65e1558c-6b09-4ddc-b520-f19f4fb972af",
    scryfall_id = "2f51f88f-f662-4572-a371-9a77718ed079",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Blue Elemental Blast",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
