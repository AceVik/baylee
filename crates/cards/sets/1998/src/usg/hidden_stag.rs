//! Hidden Stag — {1}{G} — Enchantment
//! Oracle: Whenever an opponent plays a land, if this permanent is an enchantment, it becomes a 3/2 Elk Beast creature.
//! Oracle: Whenever you play a land, if this permanent is a creature, it becomes an enchantment.
//! Set: USG #265 — Urza's Saga | Scryfall ID: 0501e115-ef61-4db5-bdb3-36dc4334431c | Oracle ID: 70c32155-dc07-45cb-bceb-e19d3ebc915d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HIDDEN_STAG,
    oracle_id = "70c32155-dc07-45cb-bceb-e19d3ebc915d",
    scryfall_id = "0501e115-ef61-4db5-bdb3-36dc4334431c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Hidden Stag",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
