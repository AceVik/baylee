//! Infinite Hourglass — {4} — Artifact
//! Oracle: At the beginning of your upkeep, put a time counter on this artifact.
//! Oracle: All creatures get +1/+0 for each time counter on this artifact.
//! Oracle: {3}: Remove a time counter from this artifact. Any player may activate this ability but only during any upkeep step.
//! Set: 5ED #378 — Fifth Edition | Scryfall ID: fbafc838-7458-4542-86f3-594326ec0691 | Oracle ID: 7c59ad65-ad1f-4dc4-bcf2-ea6b2b48fad8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INFINITE_HOURGLASS,
    oracle_id = "7c59ad65-ad1f-4dc4-bcf2-ea6b2b48fad8",
    scryfall_id = "fbafc838-7458-4542-86f3-594326ec0691",
    faces = &[face!(
        name = "Infinite Hourglass",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
