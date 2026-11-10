//! Apocalypse Chime — {2} — Artifact
//! Oracle: {2}, {T}, Sacrifice this artifact: Destroy all nontoken permanents with a name originally printed in the Homelands expansion. They can't be regenerated.
//! Set: HML #101 — Homelands | Scryfall ID: cef20d8f-6e80-4fca-b6a7-541981f6a112 | Oracle ID: 995e2412-3f69-4aaf-a5d6-ae0ecbb95061
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::APOCALYPSE_CHIME,
    oracle_id = "995e2412-3f69-4aaf-a5d6-ae0ecbb95061",
    scryfall_id = "cef20d8f-6e80-4fca-b6a7-541981f6a112",
    faces = &[face!(
        name = "Apocalypse Chime",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
