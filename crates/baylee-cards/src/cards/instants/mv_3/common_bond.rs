//! Common Bond — {1}{G}{W} — Instant
//! Oracle: Put a +1/+1 counter on target creature.
//! Oracle: Put a +1/+1 counter on target creature.
//! Set: RTR #151 — Return to Ravnica | Scryfall ID: 59965953-1522-4103-9a6c-5534205d34d9 | Oracle ID: d52e9255-62e7-479f-8def-e92b96efdb89
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::COMMON_BOND,
    oracle_id = "d52e9255-62e7-479f-8def-e92b96efdb89",
    scryfall_id = "59965953-1522-4103-9a6c-5534205d34d9",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Common Bond",
        mana_cost = mana!("{1}{G}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
