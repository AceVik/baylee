//! Oath of Lim-Dûl — {3}{B} — Enchantment
//! Oracle: Whenever you lose life, for each 1 life you lost, sacrifice a permanent other than this enchantment unless you discard a card. (Damage dealt to you causes you to lose life.)
//! Oracle: {B}{B}: Draw a card.
//! Set: ICE #156 — Ice Age | Scryfall ID: f16df768-06de-43a0-b548-44fb0887490b | Oracle ID: 64e163c2-2378-4e4c-aefe-5e0d2fd19988
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OATH_OF_LIM_DUL,
    oracle_id = "64e163c2-2378-4e4c-aefe-5e0d2fd19988",
    scryfall_id = "f16df768-06de-43a0-b548-44fb0887490b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Oath of Lim-Dûl",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
