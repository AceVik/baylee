//! Floodwater Dam — {3} — Artifact
//! Oracle: {X}{X}{1}, {T}: Tap X target lands.
//! Set: ME4 #200 — Masters Edition IV | Scryfall ID: 0f1bc264-1f28-4bd4-be23-baae9734c3d9 | Oracle ID: a3947473-0f57-4937-bb1c-f20969144415
use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLOODWATER_DAM,
    oracle_id = "a3947473-0f57-4937-bb1c-f20969144415",
    scryfall_id = "0f1bc264-1f28-4bd4-be23-baae9734c3d9",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Floodwater Dam",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{X}{X}{1}", TapSelf),
        &[Effect::TapTarget],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::LAND))),
    )],
);
