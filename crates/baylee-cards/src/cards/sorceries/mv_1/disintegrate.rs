//! Disintegrate — {X}{R} — Sorcery
//! Oracle: Disintegrate deals X damage to any target. If it's a creature, it can't be regenerated this turn, and if it would die this turn, exile it instead.
//! Set: TSB #58 — Time Spiral Timeshifted | Scryfall ID: a83420d1-c51a-4fb7-8f1d-b376a0083d95 | Oracle ID: 92d6af2f-728e-4e41-87cb-5c90878a2f2f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISINTEGRATE,
    oracle_id = "92d6af2f-728e-4e41-87cb-5c90878a2f2f",
    scryfall_id = "a83420d1-c51a-4fb7-8f1d-b376a0083d95",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Disintegrate",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
