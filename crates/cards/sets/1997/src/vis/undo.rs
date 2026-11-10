//! Undo — {1}{U}{U} — Sorcery
//! Oracle: Return two target creatures to their owners' hands.
//! Set: S99 #58 — Starter 1999 | Scryfall ID: 12b944ea-add5-4806-a33e-264cc96a4a0e | Oracle ID: b446c3bd-3c7d-4d59-ba59-3dd80818465e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNDO,
    oracle_id = "b446c3bd-3c7d-4d59-ba59-3dd80818465e",
    scryfall_id = "12b944ea-add5-4806-a33e-264cc96a4a0e",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Undo",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::bounce(TargetSpec::Object(&Filter::CREATURE))],
        targets = Some(TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), 2)),
    )],
);
