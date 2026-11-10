//! Rout — {3}{W}{W} — Sorcery
//! Oracle: You may cast this spell as though it had flash if you pay {2} more to cast it. (You may cast it any time you could cast an instant.)
//! Oracle: Destroy all creatures. They can't be regenerated.
//! Set: C21 #101 — Commander 2021 | Scryfall ID: 15ce585a-67fc-4e7a-92ff-a27292179dfa | Oracle ID: d69aa624-23d1-406a-b7e0-9140a6f57929
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ROUT,
    oracle_id = "d69aa624-23d1-406a-b7e0-9140a6f57929",
    scryfall_id = "15ce585a-67fc-4e7a-92ff-a27292179dfa",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Rout",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
