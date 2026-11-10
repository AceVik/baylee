//! Oblation — {2}{W} — Instant
//! Oracle: The owner of target nonland permanent shuffles it into their library, then draws two cards.
//! Set: C21 #97 — Commander 2021 | Scryfall ID: dcfdea72-4736-4442-987b-173f6885528a | Oracle ID: 860e744d-899c-43c7-84bb-d52c69d2f45a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OBLATION,
    oracle_id = "860e744d-899c-43c7-84bb-d52c69d2f45a",
    scryfall_id = "dcfdea72-4736-4442-987b-173f6885528a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Oblation",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
