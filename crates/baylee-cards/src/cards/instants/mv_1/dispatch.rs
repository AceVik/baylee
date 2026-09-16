//! Dispatch — {W} — Instant
//! Oracle: Tap target creature.
//! Oracle: Metalcraft — If you control three or more artifacts, exile that creature.
//! Set: MSC #130 — Marvel Super Heroes Commander | Scryfall ID: 99d30a21-b003-4a17-a4c5-97811f230896 | Oracle ID: 133c99c0-3652-410f-8100-68015a47af9f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISPATCH,
    oracle_id = "133c99c0-3652-410f-8100-68015a47af9f",
    scryfall_id = "99d30a21-b003-4a17-a4c5-97811f230896",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Dispatch",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
