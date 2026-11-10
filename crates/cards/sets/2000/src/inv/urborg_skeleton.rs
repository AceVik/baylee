//! Urborg Skeleton — {B} — Creature — Skeleton
//! Oracle: Kicker {3} (You may pay an additional {3} as you cast this spell.)
//! Oracle: {B}: Regenerate this creature.
//! Oracle: If this creature was kicked, it enters with a +1/+1 counter on it.
//! Set: INV #134 — Invasion | Scryfall ID: 6e522a62-fbca-4362-9006-d4356c525704 | Oracle ID: 712d7eb4-4044-4852-a5da-57635d3086b0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URBORG_SKELETON,
    oracle_id = "712d7eb4-4044-4852-a5da-57635d3086b0",
    scryfall_id = "6e522a62-fbca-4362-9006-d4356c525704",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Urborg Skeleton",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SKELETON],
        power = Some(0),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
