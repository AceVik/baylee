//! Ruby Medallion — {2} — Artifact
//! Oracle: Red spells you cast cost {1} less to cast.
//! Set: MH3 #295 — Modern Horizons 3 | Scryfall ID: 630febf5-58ff-4dab-a5a8-575ebc5435a6 | Oracle ID: 0c367958-729e-416d-988b-098e90e7a1fd
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static RED_SPELL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::Red])),
]);

card!(
    index = index::RUBY_MEDALLION,
    oracle_id = "0c367958-729e-416d-988b-098e90e7a1fd",
    scryfall_id = "630febf5-58ff-4dab-a5a8-575ebc5435a6",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ruby Medallion",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(RED_SPELL, Modifier::SpellsCostLess(1))],
);
