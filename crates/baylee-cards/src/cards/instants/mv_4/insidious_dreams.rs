//! Insidious Dreams — {3}{B} — Instant
//! Oracle: As an additional cost to cast this spell, discard X cards.
//! Oracle: Search your library for X cards, then shuffle and put those cards on top in any order.
//! Set: TOR #66 — Torment | Scryfall ID: e8a29622-23a6-42c7-8e56-690613572c94 | Oracle ID: 4badd4e0-e5d7-4516-b220-cea2b02c79aa
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INSIDIOUS_DREAMS,
    oracle_id = "4badd4e0-e5d7-4516-b220-cea2b02c79aa",
    scryfall_id = "e8a29622-23a6-42c7-8e56-690613572c94",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Insidious Dreams",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
