//! Yawgmoth's Bargain — {4}{B}{B} — Enchantment
//! Oracle: Skip your draw step.
//! Oracle: Pay 1 life: Draw a card.
//! Set: VMA #147 — Vintage Masters | Scryfall ID: eef31b59-45bb-4208-8957-1564902c8507 | Oracle ID: f7f76f39-a0de-4bda-86b6-0f291892fcec
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::YAWGMOTH_S_BARGAIN,
    oracle_id = "f7f76f39-a0de-4bda-86b6-0f291892fcec",
    scryfall_id = "eef31b59-45bb-4208-8957-1564902c8507",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Yawgmoth's Bargain",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
