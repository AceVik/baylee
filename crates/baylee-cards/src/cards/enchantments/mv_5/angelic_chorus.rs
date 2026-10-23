//! Angelic Chorus — {3}{W}{W} — Enchantment
//! Oracle: Whenever a creature you control enters, you gain life equal to its toughness.
//! Set: BBD #87 — Battlebond | Scryfall ID: d9e966cb-bb42-49d8-b351-114619b31497 | Oracle ID: e66b12ff-45da-46dc-9148-bd8ea9b7fde0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ANGELIC_CHORUS,
    oracle_id = "e66b12ff-45da-46dc-9148-bd8ea9b7fde0",
    scryfall_id = "d9e966cb-bb42-49d8-b351-114619b31497",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Angelic Chorus",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
