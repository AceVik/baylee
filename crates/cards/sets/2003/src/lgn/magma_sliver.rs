//! Magma Sliver — {3}{R} — Creature — Sliver
//! Oracle: All Slivers have "{T}: Target Sliver creature gets +X/+0 until end of turn, where X is the number of Slivers on the battlefield."
//! Set: LGN #107 — Legions | Scryfall ID: 9091d908-456f-4127-857d-b22fdb4f2fd9 | Oracle ID: 008ac258-763c-4379-bfcd-b9cc4d7296dd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MAGMA_SLIVER,
    oracle_id = "008ac258-763c-4379-bfcd-b9cc4d7296dd",
    scryfall_id = "9091d908-456f-4127-857d-b22fdb4f2fd9",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Magma Sliver",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
