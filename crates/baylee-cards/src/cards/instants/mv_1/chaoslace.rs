//! Chaoslace — {R} — Instant
//! Oracle: Target spell or permanent becomes red. (Its mana symbols remain unchanged.)
//! Set: 4ED #182 — Fourth Edition | Scryfall ID: 476180df-8b88-4ead-b6c5-6ccb3e8a2cfd | Oracle ID: 08842aa3-f923-46e9-a106-f542331e9cc1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHAOSLACE,
    oracle_id = "08842aa3-f923-46e9-a106-f542331e9cc1",
    scryfall_id = "476180df-8b88-4ead-b6c5-6ccb3e8a2cfd",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Chaoslace",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
