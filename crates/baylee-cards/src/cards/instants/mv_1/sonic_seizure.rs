//! Sonic Seizure — {R} — Instant
//! Oracle: As an additional cost to cast this spell, discard a card at random.
//! Oracle: Sonic Seizure deals 3 damage to any target.
//! Set: TOR #115 — Torment | Scryfall ID: 98eb9371-aa20-4790-baf8-a1ad95de39de | Oracle ID: f43c68f1-97ac-47a4-82d3-a41277259388
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SONIC_SEIZURE,
    oracle_id = "f43c68f1-97ac-47a4-82d3-a41277259388",
    scryfall_id = "98eb9371-aa20-4790-baf8-a1ad95de39de",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Sonic Seizure",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
