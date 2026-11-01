//! Flame Wave — {3}{R}{R}{R}{R} — Sorcery
//! Oracle: Flame Wave deals 4 damage to target player or planeswalker and each creature that player or that planeswalker's controller controls.
//! Set: TPR #131 — Tempest Remastered | Scryfall ID: f592d08e-db49-4cd5-bc45-aea56da3f828 | Oracle ID: cee8ec5f-e36d-4bca-9b19-cb33113835b1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLAME_WAVE,
    oracle_id = "cee8ec5f-e36d-4bca-9b19-cb33113835b1",
    scryfall_id = "f592d08e-db49-4cd5-bc45-aea56da3f828",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Flame Wave",
        mana_cost = mana!("{3}{R}{R}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
