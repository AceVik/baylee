//! Mishra's Helix — {5} — Artifact
//! Oracle: {X}, {T}: Tap X target lands.
//! Set: USG #302 — Urza's Saga | Scryfall ID: 5c62507f-465d-45e9-96bf-06671d19e79e | Oracle ID: 6b1552f2-df64-485a-8509-b1916edf19cf
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MISHRA_S_HELIX,
    oracle_id = "6b1552f2-df64-485a-8509-b1916edf19cf",
    scryfall_id = "5c62507f-465d-45e9-96bf-06671d19e79e",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mishra's Helix",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[Effect::TapTarget],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::LAND))),
    )],
);
