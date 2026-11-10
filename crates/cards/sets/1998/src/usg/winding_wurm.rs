//! Winding Wurm — {4}{G} — Creature — Wurm
//! Oracle: Echo {4}{G} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Set: USG #285 — Urza's Saga | Scryfall ID: ed75dc43-172c-4302-8807-23bfdd65baf4 | Oracle ID: 8f788188-d901-44f3-9fec-e2e5d0263140
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WINDING_WURM,
    oracle_id = "8f788188-d901-44f3-9fec-e2e5d0263140",
    scryfall_id = "ed75dc43-172c-4302-8807-23bfdd65baf4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Winding Wurm",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WURM],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[AbilityDef::Echo {
        cost: mana!("{4}{G}"),
    }],
);
