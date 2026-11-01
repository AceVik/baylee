//! Preferred Selection — {2}{G}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, look at the top two cards of your library. You may sacrifice this enchantment and pay {2}{G}{G}. If you do, put one of those cards into your hand. If you don't, put one of those cards on the bottom of your library.
//! Set: MIR #233 — Mirage | Scryfall ID: 59ce9668-78ef-44a5-ba9c-49fa740b8cb5 | Oracle ID: 1d05d2d6-3ea1-4d8a-9f1c-6fc3f5e38ea9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PREFERRED_SELECTION,
    oracle_id = "1d05d2d6-3ea1-4d8a-9f1c-6fc3f5e38ea9",
    scryfall_id = "59ce9668-78ef-44a5-ba9c-49fa740b8cb5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Preferred Selection",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
