//! Emerald Medallion — {2} — Artifact
//! Oracle: Green spells you cast cost {1} less to cast.
//! Set: MH3 #291 — Modern Horizons 3 | Scryfall ID: 5ddf44fe-9e4e-42a6-a6f8-11ada3ce8a94 | Oracle ID: d683024d-8fe7-4509-851b-7655fadc1de7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EMERALD_MEDALLION,
    oracle_id = "d683024d-8fe7-4509-851b-7655fadc1de7",
    scryfall_id = "5ddf44fe-9e4e-42a6-a6f8-11ada3ce8a94",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Emerald Medallion",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::And(&[
            Filter::ControlledByYou,
            Filter::HasColor(ColorSet::from_slice(&[Color::Green])),
        ]),
        Modifier::SpellsCostLess(1),
    )],
);
