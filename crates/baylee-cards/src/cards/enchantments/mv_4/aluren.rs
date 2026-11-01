//! Aluren — {2}{G}{G} — Enchantment
//! Oracle: Any player may cast creature spells with mana value 3 or less without paying their mana costs and as though they had flash.
//! Set: TPR #165 — Tempest Remastered | Scryfall ID: 03b69c39-52d8-4ee6-a3f5-60899c1fe6e6 | Oracle ID: 44e02e70-07ab-4795-8037-55185bd11d54
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALUREN,
    oracle_id = "44e02e70-07ab-4795-8037-55185bd11d54",
    scryfall_id = "03b69c39-52d8-4ee6-a3f5-60899c1fe6e6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Aluren",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
