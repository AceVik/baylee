//! Bubble Matrix — {4} — Artifact
//! Oracle: Prevent all damage that would be dealt to creatures.
//! Set: WTH #146 — Weatherlight | Scryfall ID: 0ca9c239-84ff-4527-aa23-bdb11856744c | Oracle ID: 50261dca-bfeb-40b3-95b6-9057491ef3ad
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BUBBLE_MATRIX,
    oracle_id = "50261dca-bfeb-40b3-95b6-9057491ef3ad",
    scryfall_id = "0ca9c239-84ff-4527-aa23-bdb11856744c",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bubble Matrix",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::CREATURE,
        Modifier::PreventDamageFrom(&Filter::Any)
    )],
);
