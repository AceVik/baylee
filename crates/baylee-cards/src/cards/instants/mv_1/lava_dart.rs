//! Lava Dart — {R} — Instant
//! Oracle: Lava Dart deals 1 damage to any target.
//! Oracle: Flashback—Sacrifice a Mountain. (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: MH1 #134 — Modern Horizons | Scryfall ID: b16dd041-451d-4914-8c46-aa315a90d802 | Oracle ID: e48891e3-30a2-4fc8-a858-cec33c6e4ab5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAVA_DART,
    oracle_id = "e48891e3-30a2-4fc8-a858-cec33c6e4ab5",
    scryfall_id = "b16dd041-451d-4914-8c46-aa315a90d802",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Lava Dart",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
