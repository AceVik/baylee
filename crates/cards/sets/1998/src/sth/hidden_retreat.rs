//! Hidden Retreat — {2}{W} — Enchantment
//! Oracle: Put a card from your hand on top of your library: Prevent all damage that would be dealt by target instant or sorcery spell this turn.
//! Set: STH #6 — Stronghold | Scryfall ID: 5be86f1f-c85a-4396-aa16-a39fbf493c96 | Oracle ID: a116329a-343e-4f10-a122-38bf8b5ac2c8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HIDDEN_RETREAT,
    oracle_id = "a116329a-343e-4f10-a122-38bf8b5ac2c8",
    scryfall_id = "5be86f1f-c85a-4396-aa16-a39fbf493c96",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Hidden Retreat",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
