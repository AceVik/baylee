//! Pandemonium — {3}{R} — Enchantment
//! Oracle: Whenever a creature enters, that creature's controller may have it deal damage equal to its power to any target of their choice.
//! Set: TPR #149 — Tempest Remastered | Scryfall ID: 45ad514e-28a6-4816-b764-69ef89e117f1 | Oracle ID: 073551ca-2e84-4d49-8484-9890718cc483
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PANDEMONIUM,
    oracle_id = "073551ca-2e84-4d49-8484-9890718cc483",
    scryfall_id = "45ad514e-28a6-4816-b764-69ef89e117f1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Pandemonium",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
