//! Ancestral Memories — {2}{U}{U}{U} — Sorcery
//! Oracle: Look at the top seven cards of your library. Put two of them into your hand and the rest into your graveyard.
//! Set: 7ED #59 — Seventh Edition | Scryfall ID: 100f0ca8-a66a-452f-be5f-17f631ba0ee0 | Oracle ID: 95a2802a-2621-40c3-84f8-51e8aad7b6f0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANCESTRAL_MEMORIES,
    oracle_id = "95a2802a-2621-40c3-84f8-51e8aad7b6f0",
    scryfall_id = "100f0ca8-a66a-452f-be5f-17f631ba0ee0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Ancestral Memories",
        mana_cost = mana!("{2}{U}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
