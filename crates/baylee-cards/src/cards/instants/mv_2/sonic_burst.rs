//! Sonic Burst — {1}{R} — Instant
//! Oracle: As an additional cost to cast this spell, discard a card at random.
//! Oracle: Sonic Burst deals 4 damage to any target.
//! Set: EXO #103 — Exodus | Scryfall ID: 05530d5a-dcb6-403e-9e35-224c7b5cf615 | Oracle ID: 144d42b8-5c44-4a36-9028-75229047e56c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SONIC_BURST,
    oracle_id = "144d42b8-5c44-4a36-9028-75229047e56c",
    scryfall_id = "05530d5a-dcb6-403e-9e35-224c7b5cf615",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Sonic Burst",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
