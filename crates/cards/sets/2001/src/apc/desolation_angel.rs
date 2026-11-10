//! Desolation Angel — {3}{B}{B} — Creature — Angel
//! Oracle: Kicker {W}{W} (You may pay an additional {W}{W} as you cast this spell.)
//! Oracle: Flying
//! Oracle: When this creature enters, destroy all lands you control. If it was kicked, destroy all lands instead.
//! Set: APC #38 — Apocalypse | Scryfall ID: 445127d4-8afb-47cf-b2a1-564540b1fdae | Oracle ID: 87f82107-eaba-484a-800d-e61a0cc5e1f2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DESOLATION_ANGEL,
    oracle_id = "87f82107-eaba-484a-800d-e61a0cc5e1f2",
    scryfall_id = "445127d4-8afb-47cf-b2a1-564540b1fdae",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(
        name = "Desolation Angel",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(5),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
