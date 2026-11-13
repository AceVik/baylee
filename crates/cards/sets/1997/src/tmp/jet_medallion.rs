//! Jet Medallion — {2} — Artifact
//! Oracle: Black spells you cast cost {1} less to cast.
//! Set: MH3 #292 — Modern Horizons 3 | Scryfall ID: d423365c-a32c-4060-b178-3221e3200387 | Oracle ID: bccb06f1-8839-4104-9ffa-c79a817f8378
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JET_MEDALLION,
    oracle_id = "bccb06f1-8839-4104-9ffa-c79a817f8378",
    scryfall_id = "d423365c-a32c-4060-b178-3221e3200387",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jet Medallion",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::And(&[
            Filter::ControlledByYou,
            Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
        ]),
        Modifier::SpellsCostLess(1),
    )],
);
