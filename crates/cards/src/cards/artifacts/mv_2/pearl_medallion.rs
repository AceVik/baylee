//! Pearl Medallion — {2} — Artifact
//! Oracle: White spells you cast cost {1} less to cast.
//! Set: MH3 #294 — Modern Horizons 3 | Scryfall ID: 1225264f-b298-4b56-b208-c5da87bd6867 | Oracle ID: 19015380-1332-4960-8cc6-0732009525a2
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static WHITE_SPELL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);

card!(
    index = index::PEARL_MEDALLION,
    oracle_id = "19015380-1332-4960-8cc6-0732009525a2",
    scryfall_id = "1225264f-b298-4b56-b208-c5da87bd6867",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pearl Medallion",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(WHITE_SPELL, Modifier::SpellsCostLess(1))],
);
