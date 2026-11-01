//! Crypt Sliver — {1}{B} — Creature — Sliver
//! Oracle: All Slivers have "{T}: Regenerate target Sliver."
//! Set: CMM #863 — Commander Masters | Scryfall ID: 2b90c9a9-098b-463a-8588-b078fb0364dd | Oracle ID: b0bc2d60-20d0-4ca2-be86-65b297226ac7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRYPT_SLIVER,
    oracle_id = "b0bc2d60-20d0-4ca2-be86-65b297226ac7",
    scryfall_id = "2b90c9a9-098b-463a-8588-b078fb0364dd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Crypt Sliver",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
