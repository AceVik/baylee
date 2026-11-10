//! Essence Sliver — {3}{W} — Creature — Sliver
//! Oracle: Whenever a Sliver deals damage, its controller gains that much life.
//! Set: TSB #8 — Time Spiral Timeshifted | Scryfall ID: 54e01152-944b-47dc-838e-753925ac64e2 | Oracle ID: 1f35f5e3-ddd1-4293-96f8-1181bbda77eb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ESSENCE_SLIVER,
    oracle_id = "1f35f5e3-ddd1-4293-96f8-1181bbda77eb",
    scryfall_id = "54e01152-944b-47dc-838e-753925ac64e2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Essence Sliver",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
