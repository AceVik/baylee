//! Chandra's Outrage — {2}{R}{R} — Instant
//! Oracle: Chandra's Outrage deals 4 damage to target creature and 2 damage to that creature's controller.
//! Set: M20 #130 — Core Set 2020 | Scryfall ID: edbfe793-6644-43ed-bbe1-e122c01c5e53 | Oracle ID: 47437865-0032-4f47-b0ab-034cc841bb84
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHANDRA_S_OUTRAGE,
    oracle_id = "47437865-0032-4f47-b0ab-034cc841bb84",
    scryfall_id = "edbfe793-6644-43ed-bbe1-e122c01c5e53",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Chandra's Outrage",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
