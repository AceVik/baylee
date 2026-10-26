//! Mercadia's Downfall — {2}{R} — Instant
//! Oracle: Each attacking creature gets +1/+0 until end of turn for each nonbasic land defending player controls.
//! Set: MMQ #205 — Mercadian Masques | Scryfall ID: 14507fe6-80a9-4ed4-bf3e-4656f3d377c0 | Oracle ID: 2c1201b2-5036-4e68-b36d-cdaf54b4993d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MERCADIA_S_DOWNFALL,
    oracle_id = "2c1201b2-5036-4e68-b36d-cdaf54b4993d",
    scryfall_id = "14507fe6-80a9-4ed4-bf3e-4656f3d377c0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Mercadia's Downfall",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
