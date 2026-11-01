//! Inspirit — {2}{W} — Instant
//! Oracle: Untap target creature. It gets +2/+4 until end of turn.
//! Set: 9ED #22 — Ninth Edition | Scryfall ID: 5d30356b-ab7b-4366-9f90-15703d5c3d10 | Oracle ID: 90fd82e8-f638-4ba4-a03c-a30d46f93a39
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INSPIRIT,
    oracle_id = "90fd82e8-f638-4ba4-a03c-a30d46f93a39",
    scryfall_id = "5d30356b-ab7b-4366-9f90-15703d5c3d10",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Inspirit",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
