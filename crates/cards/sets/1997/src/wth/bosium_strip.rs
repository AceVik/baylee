//! Bösium Strip — {3} — Artifact
//! Oracle: {3}, {T}: Until end of turn, you may cast instant and sorcery spells from the top of your graveyard. If a spell cast this way would be put into a graveyard, exile it instead.
//! Set: WTH #147 — Weatherlight | Scryfall ID: 3884bede-df28-42e8-9ac9-ae03118b1985 | Oracle ID: 2fa6f1b1-00af-433f-b8e1-36db99cd9bba
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BOSIUM_STRIP,
    oracle_id = "2fa6f1b1-00af-433f-b8e1-36db99cd9bba",
    scryfall_id = "3884bede-df28-42e8-9ac9-ae03118b1985",
    faces = &[face!(
        name = "Bösium Strip",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
