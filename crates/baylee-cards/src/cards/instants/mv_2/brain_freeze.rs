//! Brain Freeze — {1}{U} — Instant
//! Oracle: Target player mills three cards.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)
//! Set: VMA #57 — Vintage Masters | Scryfall ID: 3a2d7cf9-dddb-4de3-b4f2-c52e3ec8fb4b | Oracle ID: 464c0150-3dbc-403b-9ada-fef25ab1f29d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRAIN_FREEZE,
    oracle_id = "464c0150-3dbc-403b-9ada-fef25ab1f29d",
    scryfall_id = "3a2d7cf9-dddb-4de3-b4f2-c52e3ec8fb4b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Brain Freeze",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
