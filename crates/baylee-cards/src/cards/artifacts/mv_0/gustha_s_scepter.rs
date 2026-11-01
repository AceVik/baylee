//! Gustha's Scepter — {0} — Artifact
//! Oracle: {T}: Exile a card from your hand face down. You may look at it for as long as it remains exiled.
//! Oracle: {T}: Return a card you own exiled with this artifact to your hand.
//! Oracle: When you lose control of this artifact, put all cards exiled with this artifact into their owner's graveyard.
//! Set: ME2 #209 — Masters Edition II | Scryfall ID: 7d4f3248-2b3f-42b3-9a9f-77309bb46c34 | Oracle ID: 120e9013-b07b-49d9-8b86-f6c26315581d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GUSTHA_S_SCEPTER,
    oracle_id = "120e9013-b07b-49d9-8b86-f6c26315581d",
    scryfall_id = "7d4f3248-2b3f-42b3-9a9f-77309bb46c34",
    faces = &[face!(
        name = "Gustha's Scepter",
        mana_cost = mana!("{0}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
