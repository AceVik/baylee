//! Scroll Rack — {2} — Artifact
//! Oracle: {1}, {T}: Exile any number of cards from your hand face down. Put that many cards from the top of your library into your hand. Then look at the exiled cards and put them on top of your library in any order.
//! Set: CMR #337 — Commander Legends | Scryfall ID: dfaaa58d-89bb-4cb3-96a6-b480e6f6954e | Oracle ID: 648bd03d-247e-4d6f-87a9-12a67688cbf5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SCROLL_RACK,
    oracle_id = "648bd03d-247e-4d6f-87a9-12a67688cbf5",
    scryfall_id = "dfaaa58d-89bb-4cb3-96a6-b480e6f6954e",
    faces = &[face!(
        name = "Scroll Rack",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
