//! Alms — {W} — Enchantment
//! Oracle: {1}, Exile the top card of your graveyard: Prevent the next 1 damage that would be dealt to target creature this turn.
//! Set: WTH #3 — Weatherlight | Scryfall ID: 97382dd8-2754-4ca3-8ba8-d655acaf22ac | Oracle ID: efd46a6e-a901-4224-8a29-d53a4dc7750c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALMS,
    oracle_id = "efd46a6e-a901-4224-8a29-d53a4dc7750c",
    scryfall_id = "97382dd8-2754-4ca3-8ba8-d655acaf22ac",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Alms",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
