//! Urza's Rage — {2}{R} — Instant
//! Oracle: Kicker {8}{R} (You may pay an additional {8}{R} as you cast this spell.)
//! Oracle: This spell can't be countered.
//! Oracle: Urza's Rage deals 3 damage to any target. If this spell was kicked, instead it deals 10 damage to that permanent or player and the damage can't be prevented.
//! Set: MH1 #151 — Modern Horizons | Scryfall ID: 5c91e364-c157-494e-bb1b-f2cef004900f | Oracle ID: 363f8c66-fe0c-44b9-987d-1d160e3f9c54
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::URZA_S_RAGE,
    oracle_id = "363f8c66-fe0c-44b9-987d-1d160e3f9c54",
    scryfall_id = "5c91e364-c157-494e-bb1b-f2cef004900f",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Urza's Rage",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
