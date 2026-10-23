//! Raze — {R} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a land.
//! Oracle: Destroy target land.
//! Set: USG #207 — Urza's Saga | Scryfall ID: 56d51b3c-24e9-41b6-b7cd-c70329e498ca | Oracle ID: 02fde433-955d-49df-bd72-270b6feac9e7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAZE,
    oracle_id = "02fde433-955d-49df-bd72-270b6feac9e7",
    scryfall_id = "56d51b3c-24e9-41b6-b7cd-c70329e498ca",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Raze",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::YOUR_LAND)],
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::LAND))],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))),
    ),],
);
