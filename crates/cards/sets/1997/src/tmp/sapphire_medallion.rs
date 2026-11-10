//! Sapphire Medallion — {2} — Artifact
//! Oracle: Blue spells you cast cost {1} less to cast.
//! Set: MH3 #296 — Modern Horizons 3 | Scryfall ID: a77850b2-6478-449e-bdb9-5b92650e3504 | Oracle ID: f1ef5698-14bd-42bf-aae5-2870699e4186
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static BLUE_SPELL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);

card!(
    index = index::SAPPHIRE_MEDALLION,
    oracle_id = "f1ef5698-14bd-42bf-aae5-2870699e4186",
    scryfall_id = "a77850b2-6478-449e-bdb9-5b92650e3504",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sapphire Medallion",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(BLUE_SPELL, Modifier::SpellsCostLess(1))],
);
