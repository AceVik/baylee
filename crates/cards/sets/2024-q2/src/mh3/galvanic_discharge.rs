//! Galvanic Discharge — {R} — Instant
//! Oracle: Choose target creature or planeswalker. You get {E}{E}{E} (three energy counters), then you may pay any amount of {E}. Galvanic Discharge deals that much damage to that permanent.
//! Set: MH3 #122 — Modern Horizons 3 | Scryfall ID: 32aa6e33-221f-414c-9b51-850d97a7e051 | Oracle ID: ef0f06f1-3991-4897-bf62-d4cab7ec79a0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GALVANIC_DISCHARGE,
    oracle_id = "ef0f06f1-3991-4897-bf62-d4cab7ec79a0",
    scryfall_id = "32aa6e33-221f-414c-9b51-850d97a7e051",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Galvanic Discharge",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
