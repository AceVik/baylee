//! Candelabra of Tawnos — {1} — Artifact
//! Oracle: {X}, {T}: Untap X target lands.
//! Set: ME4 #187 — Masters Edition IV | Scryfall ID: 5e8416b8-aae1-4599-9e22-650dd86aefcb | Oracle ID: c7c7bffa-442d-4ba5-b778-ad394c192f27
// IMPLEMENTED — {X}, {T} untaps X target lands; X is announced for the
// activation and bounds the target count (`TargetReq::x_targets`).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CANDELABRA_OF_TAWNOS,
    oracle_id = "c7c7bffa-442d-4ba5-b778-ad394c192f27",
    scryfall_id = "5e8416b8-aae1-4599-9e22-650dd86aefcb",
    faces = &[face!(
        name = "Candelabra of Tawnos",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[Effect::UntapTarget],
        targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::LAND)))
    )],
);
