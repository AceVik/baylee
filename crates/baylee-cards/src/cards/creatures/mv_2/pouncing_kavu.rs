//! Pouncing Kavu — {1}{R} — Creature — Kavu
//! Oracle: Kicker {2}{R} (You may pay an additional {2}{R} as you cast this spell.)
//! Oracle: First strike
//! Oracle: If this creature was kicked, it enters with two +1/+1 counters on it and with haste.
//! Set: INV #158 — Invasion | Scryfall ID: 7e6e2e49-7bde-43c1-8caf-43d237dfc052 | Oracle ID: 5f9a968d-cc23-46d3-b28f-3779dd89c531
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::POUNCING_KAVU,
    oracle_id = "5f9a968d-cc23-46d3-b28f-3779dd89c531",
    scryfall_id = "7e6e2e49-7bde-43c1-8caf-43d237dfc052",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Pouncing Kavu",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KAVU],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
