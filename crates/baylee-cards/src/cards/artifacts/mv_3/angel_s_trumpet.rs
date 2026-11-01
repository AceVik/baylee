//! Angel's Trumpet — {3} — Artifact
//! Oracle: All creatures have vigilance.
//! Oracle: At the beginning of each player's end step, tap all untapped creatures that player controls that didn't attack this turn. This artifact deals damage to the player equal to the number of creatures tapped this way.
//! Set: ULG #121 — Urza's Legacy | Scryfall ID: 0c7b248a-3c74-4592-b357-47989568298c | Oracle ID: f187f766-3696-419e-9865-58023a46446b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANGEL_S_TRUMPET,
    oracle_id = "f187f766-3696-419e-9865-58023a46446b",
    scryfall_id = "0c7b248a-3c74-4592-b357-47989568298c",
    faces = &[face!(
        name = "Angel's Trumpet",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
