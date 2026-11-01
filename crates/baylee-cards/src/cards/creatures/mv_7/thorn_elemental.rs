//! Thorn Elemental — {5}{G}{G} — Creature — Elemental
//! Oracle: You may have this creature assign its combat damage as though it weren't blocked.
//! Set: DOM #185 — Dominaria | Scryfall ID: da901037-20e6-4445-8e7e-1ccd2e8b13ae | Oracle ID: 9cdf840d-314e-46ef-9c04-74f145d9e65b
// NOT SUPPORTED: You may have this creature assign its combat damage as though it weren't blocked — optional unblocked combat-damage assignment is not in the DSL
// PARTIAL — {5}{G}{G} 7/7 Elemental body; "You may have this creature assign
// its combat damage as though it weren't blocked" is not in the DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THORN_ELEMENTAL,
    oracle_id = "9cdf840d-314e-46ef-9c04-74f145d9e65b",
    scryfall_id = "da901037-20e6-4445-8e7e-1ccd2e8b13ae",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "You may have this creature assign its combat damage as though it weren't blocked — optional unblocked combat-damage assignment is not in the DSL",
    ),
    faces = &[face!(
        name = "Thorn Elemental",
        mana_cost = mana!("{5}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(7),
        toughness = Some(7),
    ),],
);
