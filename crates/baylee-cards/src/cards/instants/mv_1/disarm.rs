//! Disarm — {U} — Instant
//! Oracle: Unattach all Equipment from target creature.
//! Set: MRD #32 — Mirrodin | Scryfall ID: 427c6350-af52-45f1-8024-5f31aa62a0d0 | Oracle ID: 44d54814-2eeb-4df8-b8d3-789f0bcb307a
use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISARM,
    oracle_id = "44d54814-2eeb-4df8-b8d3-789f0bcb307a",
    scryfall_id = "427c6350-af52-45f1-8024-5f31aa62a0d0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Disarm",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::UnattachAllEquipmentFromTarget],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
