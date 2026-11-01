//! Thran Tome — {4} — Artifact — Book
//! Oracle: {5}, {T}: Reveal the top three cards of your library. Target opponent chooses one of those cards. Put that card into your graveyard, then draw two cards.
//! Set: WTH #160 — Weatherlight | Scryfall ID: 63db7360-fe6e-430f-bfee-a2f80bcb6fec | Oracle ID: 78d2a3fd-864c-49f3-b531-c71dc3b35977
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THRAN_TOME,
    oracle_id = "78d2a3fd-864c-49f3-b531-c71dc3b35977",
    scryfall_id = "63db7360-fe6e-430f-bfee-a2f80bcb6fec",
    faces = &[face!(
        name = "Thran Tome",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::BOOK],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
