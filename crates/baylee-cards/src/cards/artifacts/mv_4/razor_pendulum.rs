//! Razor Pendulum — {4} — Artifact
//! Oracle: At the beginning of each player's end step, if that player has 5 or less life, this artifact deals 2 damage to that player.
//! Set: MIR #317 — Mirage | Scryfall ID: 72562860-6097-4c63-9858-ae130805f4d6 | Oracle ID: bf7c16db-01c9-41ed-bda6-c5326198ce77
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAZOR_PENDULUM,
    oracle_id = "bf7c16db-01c9-41ed-bda6-c5326198ce77",
    scryfall_id = "72562860-6097-4c63-9858-ae130805f4d6",
    faces = &[face!(
        name = "Razor Pendulum",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
