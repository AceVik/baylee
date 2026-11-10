//! Phyrexian Processor — {4} — Artifact
//! Oracle: As this artifact enters, pay any amount of life.
//! Oracle: {4}, {T}: Create an X/X black Phyrexian Minion creature token, where X is the life paid as this artifact entered.
//! Set: USG #306 — Urza's Saga | Scryfall ID: 6875ce99-badd-44da-8e5d-509600efa1d0 | Oracle ID: 36c800cb-b1ca-4432-ad3c-4d8b90337f4c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PHYREXIAN_PROCESSOR,
    oracle_id = "36c800cb-b1ca-4432-ad3c-4d8b90337f4c",
    scryfall_id = "6875ce99-badd-44da-8e5d-509600efa1d0",
    faces = &[face!(
        name = "Phyrexian Processor",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
