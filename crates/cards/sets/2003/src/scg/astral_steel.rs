//! Astral Steel — {2}{W} — Instant
//! Oracle: Target creature gets +1/+2 until end of turn.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)
//! Set: SCG #2 — Scourge | Scryfall ID: 64f836d3-52c8-4628-b18a-8c8fb67969c9 | Oracle ID: ef8ac291-3789-4147-a11f-c62fd8566dbe
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASTRAL_STEEL,
    oracle_id = "ef8ac291-3789-4147-a11f-c62fd8566dbe",
    scryfall_id = "64f836d3-52c8-4628-b18a-8c8fb67969c9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Astral Steel",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
