//! Burn the Accursed — {4}{R} — Instant
//! Oracle: Burn the Accursed deals 5 damage to target creature and 2 damage to that creature's controller. If that creature would die this turn, exile it instead.
//! Set: MID #132 — Innistrad: Midnight Hunt | Scryfall ID: ff4d4e6b-564d-46da-8e32-09ed08c8ddc5 | Oracle ID: b5aae42b-3fde-4f10-b85e-882c528badef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BURN_THE_ACCURSED,
    oracle_id = "b5aae42b-3fde-4f10-b85e-882c528badef",
    scryfall_id = "ff4d4e6b-564d-46da-8e32-09ed08c8ddc5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Burn the Accursed",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
