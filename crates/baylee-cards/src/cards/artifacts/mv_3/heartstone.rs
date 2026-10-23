//! Heartstone — {3} — Artifact
//! Oracle: Activated abilities of creatures cost {1} less to activate. This effect can't reduce the mana in that cost to less than one mana.
//! Set: STH #134 — Stronghold | Scryfall ID: 0ed17325-6d2f-404e-b13f-d2d419d522b7 | Oracle ID: 8f8ebf49-37c5-4da0-9cee-84ad22ab5d37
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HEARTSTONE,
    oracle_id = "8f8ebf49-37c5-4da0-9cee-84ad22ab5d37",
    scryfall_id = "0ed17325-6d2f-404e-b13f-d2d419d522b7",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Heartstone",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::CREATURE,
        Modifier::AbilitiesCostLess {
            amount: 1,
            minimum: 1,
        }
    )],
);
