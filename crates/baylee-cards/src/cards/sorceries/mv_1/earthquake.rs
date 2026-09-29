//! Earthquake — {X}{R} — Sorcery
//! Oracle: Earthquake deals X damage to each creature without flying and each player.
//! Set: CM2 #95 — Commander Anthology Volume II | Scryfall ID: 7cf2f3c7-66b2-48f6-97ef-09150903df7e | Oracle ID: 9a40614b-50a3-422c-849e-53c8b7d3d204
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EARTHQUAKE,
    oracle_id = "9a40614b-50a3-422c-849e-53c8b7d3d204",
    scryfall_id = "7cf2f3c7-66b2-48f6-97ef-09150903df7e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Earthquake",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
