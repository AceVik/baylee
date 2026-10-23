//! Three Wishes — {1}{U}{U} — Instant
//! Oracle: Exile the top three cards of your library face down. You may look at those cards for as long as they remain exiled. Until your next turn, you may play those cards. At the beginning of your next upkeep, put any of those cards you didn't play into your graveyard.
//! Set: VIS #45 — Visions | Scryfall ID: dbb2b253-7023-44d1-963b-eae98d48f498 | Oracle ID: 6b3110a5-fd38-4d09-9073-c8a6f3f17840
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THREE_WISHES,
    oracle_id = "6b3110a5-fd38-4d09-9073-c8a6f3f17840",
    scryfall_id = "dbb2b253-7023-44d1-963b-eae98d48f498",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Three Wishes",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
