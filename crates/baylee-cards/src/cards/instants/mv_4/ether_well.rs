//! Ether Well — {3}{U} — Instant
//! Oracle: Put target creature on top of its owner's library. If that creature is red, you may put it on the bottom of its owner's library instead.
//! Set: MIR #65 — Mirage | Scryfall ID: 0a2cf195-01bf-4076-a0c6-ca5403d84f7d | Oracle ID: a2b6faca-3321-45d2-a027-ff3a8ae61f10
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ETHER_WELL,
    oracle_id = "a2b6faca-3321-45d2-a027-ff3a8ae61f10",
    scryfall_id = "0a2cf195-01bf-4076-a0c6-ca5403d84f7d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Ether Well",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
