//! Telepathy — {U} — Enchantment
//! Oracle: Your opponents play with their hands revealed.
//! Set: USG #102 — Urza's Saga | Scryfall ID: 51729f36-0a0c-47fb-a3bf-22afc78df7a4 | Oracle ID: d2e647ac-717f-46e6-baa8-6a793e7bdc32
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TELEPATHY,
    oracle_id = "d2e647ac-717f-46e6-baa8-6a793e7bdc32",
    scryfall_id = "51729f36-0a0c-47fb-a3bf-22afc78df7a4",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Telepathy",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
