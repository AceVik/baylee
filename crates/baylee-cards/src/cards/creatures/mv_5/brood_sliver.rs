//! Brood Sliver — {4}{G} — Creature — Sliver
//! Oracle: Whenever a Sliver deals combat damage to a player, its controller may create a 1/1 colorless Sliver creature token.
//! Set: CMM #887 — Commander Masters | Scryfall ID: 441a387b-e18a-41c7-84bb-bb7e354f4f2d | Oracle ID: 6b1d178f-9713-4dcf-920b-ddf2c94cc427
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BROOD_SLIVER,
    oracle_id = "6b1d178f-9713-4dcf-920b-ddf2c94cc427",
    scryfall_id = "441a387b-e18a-41c7-84bb-bb7e354f4f2d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Brood Sliver",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SLIVER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
