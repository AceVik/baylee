//! Stitcher's Supplier — {B} — Creature — Zombie
//! Oracle: When this creature enters or dies, mill three cards. (Put the top three cards of your library into your graveyard.)
//! Set: TDC #196 — Tarkir: Dragonstorm Commander | Scryfall ID: 2edcde06-b326-476e-884d-770187c785fe | Oracle ID: 7fd61a18-6e4f-40c5-aa00-3d101ec1ec82
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STITCHER_S_SUPPLIER,
    oracle_id = "7fd61a18-6e4f-40c5-aa00-3d101ec1ec82",
    scryfall_id = "2edcde06-b326-476e-884d-770187c785fe",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Stitcher's Supplier",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
