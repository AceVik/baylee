//! Righteousness — {W} — Instant
//! Oracle: Target blocking creature gets +7/+7 until end of turn.
//! Set: ELD #27 — Throne of Eldraine | Scryfall ID: d2910df2-e288-4fa0-9859-c5ca35da2d55 | Oracle ID: 3f6b2f76-0364-415e-a6a2-e9e5bf31b745
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RIGHTEOUSNESS,
    oracle_id = "3f6b2f76-0364-415e-a6a2-e9e5bf31b745",
    scryfall_id = "d2910df2-e288-4fa0-9859-c5ca35da2d55",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Righteousness",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
