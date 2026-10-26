//! Ley Line — {3}{G} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player may put a +1/+1 counter on target creature of their choice.
//! Set: MMQ #256 — Mercadian Masques | Scryfall ID: f8990efd-708a-4019-bce0-2d6409ecc004 | Oracle ID: 230d79dd-15dd-431d-af22-bdc6eaf86341
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LEY_LINE,
    oracle_id = "230d79dd-15dd-431d-af22-bdc6eaf86341",
    scryfall_id = "f8990efd-708a-4019-bce0-2d6409ecc004",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Ley Line",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
