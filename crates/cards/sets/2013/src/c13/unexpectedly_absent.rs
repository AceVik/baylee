//! Unexpectedly Absent — {X}{W}{W} — Instant
//! Oracle: Put target nonland permanent into its owner's library just beneath the top X cards of that library.
//! Set: C20 #106 — Commander 2020 | Scryfall ID: a1501a42-e233-423b-b68a-1b76630bbccc | Oracle ID: e8d78a83-c932-4b55-8f75-7094c672c3a9
// IMPLEMENTED — targeted nonland tuck below exactly X cards.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::UNEXPECTEDLY_ABSENT,
    oracle_id = "e8d78a83-c932-4b55-8f75-7094c672c3a9",
    scryfall_id = "a1501a42-e233-423b-b68a-1b76630bbccc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Unexpectedly Absent",
        mana_cost = mana!("{X}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PutTargetIntoLibraryBeneathTop { count: Amount::X }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::NONLAND))),
    )],
);
