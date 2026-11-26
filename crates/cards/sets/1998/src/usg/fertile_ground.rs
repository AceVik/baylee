//! Fertile Ground — {1}{G} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Whenever enchanted land is tapped for mana, its controller adds an additional one mana of any color.
//! Set: USG #252 — Urza's Saga | Scryfall ID: 091dda35-59e5-456d-8804-61513a610aed | Oracle ID: cf14d4e5-5965-45ad-97f7-26facf2884b5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FERTILE_GROUND,
    oracle_id = "cf14d4e5-5965-45ad-97f7-26facf2884b5",
    scryfall_id = "091dda35-59e5-456d-8804-61513a610aed",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Fertile Ground",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
