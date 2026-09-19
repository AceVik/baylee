//! Gaea's Might — {G} — Instant
//! Oracle: Domain — Target creature gets +1/+1 until end of turn for each basic land type among lands you control.
//! Set: DMU #164 — Dominaria United | Scryfall ID: 89c46e9c-ae7c-466c-bb11-4f4d01ea7a63 | Oracle ID: 73b26f12-78eb-4d01-9dd6-ee643c7a80a8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GAEA_S_MIGHT,
    oracle_id = "73b26f12-78eb-4d01-9dd6-ee643c7a80a8",
    scryfall_id = "89c46e9c-ae7c-466c-bb11-4f4d01ea7a63",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Gaea's Might",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
