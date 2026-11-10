//! Mask of the Mimic — {U} — Instant
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Search your library for a card with the same name as target nontoken creature, put that card onto the battlefield, then shuffle.
//! Set: STH #37 — Stronghold | Scryfall ID: 09891c87-eb74-4174-ad46-11a7f79de859 | Oracle ID: f1a16f6d-ea11-485e-8c5b-7f919a731fe0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MASK_OF_THE_MIMIC,
    oracle_id = "f1a16f6d-ea11-485e-8c5b-7f919a731fe0",
    scryfall_id = "09891c87-eb74-4174-ad46-11a7f79de859",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mask of the Mimic",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
