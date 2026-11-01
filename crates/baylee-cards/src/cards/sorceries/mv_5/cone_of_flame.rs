//! Cone of Flame — {3}{R}{R} — Sorcery
//! Oracle: Cone of Flame deals 1 damage to any target, 2 damage to another target, and 3 damage to a third target.
//! Set: W16 #12 — Welcome Deck 2016 | Scryfall ID: 9b434a60-0875-47f4-a626-2c9bfa3c9b3b | Oracle ID: 62879f3d-ed6d-4004-8196-f3fd7f614695
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONE_OF_FLAME,
    oracle_id = "62879f3d-ed6d-4004-8196-f3fd7f614695",
    scryfall_id = "9b434a60-0875-47f4-a626-2c9bfa3c9b3b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Cone of Flame",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
