//! Jabari's Influence — {3}{W}{W} — Instant
//! Oracle: Cast this spell only after combat.
//! Oracle: Gain control of target nonartifact, nonblack creature that attacked you this turn and put a -1/-0 counter on it.
//! Set: MIR #24 — Mirage | Scryfall ID: c9f764d7-4c18-40ef-8373-ef1e2a88007e | Oracle ID: 8930b1ce-80ac-48b6-a368-ccff9fd01d38
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JABARI_S_INFLUENCE,
    oracle_id = "8930b1ce-80ac-48b6-a368-ccff9fd01d38",
    scryfall_id = "c9f764d7-4c18-40ef-8373-ef1e2a88007e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Jabari's Influence",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
