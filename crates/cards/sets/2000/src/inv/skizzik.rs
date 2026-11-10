//! Skizzik — {3}{R} — Creature — Elemental
//! Oracle: Kicker {R} (You may pay an additional {R} as you cast this spell.)
//! Oracle: Trample, haste
//! Oracle: At the beginning of the end step, if this creature wasn't kicked, sacrifice it.
//! Set: DOM #145 — Dominaria | Scryfall ID: 77af9d28-1639-47bd-b925-7f3d2eefd352 | Oracle ID: 0c77d37e-791c-415e-b369-55e93147ec34
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKIZZIK,
    oracle_id = "0c77d37e-791c-415e-b369-55e93147ec34",
    scryfall_id = "77af9d28-1639-47bd-b925-7f3d2eefd352",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Skizzik",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(5),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
