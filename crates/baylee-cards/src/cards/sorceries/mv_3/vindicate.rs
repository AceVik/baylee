//! Vindicate — {1}{W}{B} — Sorcery
//! Oracle: Destroy target permanent.
//! Set: MH2 #294 — Modern Horizons 2 | Scryfall ID: 683c4e13-525c-45c9-8832-bfe67965c34e | Oracle ID: 63c1ac21-e3d8-40c2-8c09-3f31c52992ef
// IMPLEMENTED — destroy any target permanent. The printing says nothing
// about regeneration, so the permanent's own shield applies: this comment
// claimed otherwise for as long as no shield existed.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VINDICATE,
    oracle_id = "63c1ac21-e3d8-40c2-8c09-3f31c52992ef",
    scryfall_id = "683c4e13-525c-45c9-8832-bfe67965c34e",
    faces = &[face!(
        name = "Vindicate",
        mana_cost = mana!("{1}{W}{B}"),
        types = TypeSet::SORCERY,
    )],
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::Any))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any)))
    )],
);

// Engine-level coverage via s4 scenario tests: the chosen permanent is
// destroyed (battlefield → graveyard).
