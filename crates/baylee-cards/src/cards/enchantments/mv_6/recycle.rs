//! Recycle — {4}{G}{G} — Enchantment
//! Oracle: Skip your draw step.
//! Oracle: Whenever you play a card, draw a card.
//! Oracle: Your maximum hand size is two.
//! Set: TPR #189 — Tempest Remastered | Scryfall ID: 35698c86-4552-4c04-8f51-099bb2dbce6d | Oracle ID: 8cef3ce7-8fbc-4e68-93f9-d6adfc2c2cbf
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECYCLE,
    oracle_id = "8cef3ce7-8fbc-4e68-93f9-d6adfc2c2cbf",
    scryfall_id = "35698c86-4552-4c04-8f51-099bb2dbce6d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Recycle",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
