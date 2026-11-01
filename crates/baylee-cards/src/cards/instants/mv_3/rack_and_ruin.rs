//! Rack and Ruin — {2}{R} — Instant
//! Oracle: Destroy two target artifacts.
//! Set: ULG #89 — Urza's Legacy | Scryfall ID: a46e8f6a-3a1a-4c30-9348-4b31882267eb | Oracle ID: e04552f4-41c1-40f3-996e-de796c9967e4
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RACK_AND_RUIN,
    oracle_id = "e04552f4-41c1-40f3-996e-de796c9967e4",
    scryfall_id = "a46e8f6a-3a1a-4c30-9348-4b31882267eb",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rack and Ruin",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::ARTIFACT))],
        targets = Some(TargetReq::exactly(TargetSpec::Object(&Filter::ARTIFACT), 2,)),
    )],
);
