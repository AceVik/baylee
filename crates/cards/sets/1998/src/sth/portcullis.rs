//! Portcullis — {4} — Artifact
//! Oracle: Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.
//! Set: STH #139 — Stronghold | Scryfall ID: a4de4af2-fa75-4a87-b0e1-0117727917a5 | Oracle ID: 6230c179-0263-4fa4-b0ff-f2cb621de147
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PORTCULLIS,
    oracle_id = "6230c179-0263-4fa4-b0ff-f2cb621de147",
    scryfall_id = "a4de4af2-fa75-4a87-b0e1-0117727917a5",
    faces = &[face!(
        name = "Portcullis",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
