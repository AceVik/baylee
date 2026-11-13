//! Culling Scales — {3} — Artifact
//! Oracle: At the beginning of your upkeep, destroy target nonland permanent with the lowest mana value. (If two or more permanents are tied for lowest, target any one of them.)
//! Set: MRD #160 — Mirrodin | Scryfall ID: db9a57d6-bba6-463d-ad12-6f93c035d16b | Oracle ID: ec4f2266-3e8e-4194-8f06-d95ebc543661
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static NONLAND_LOWEST: Filter = Filter::LowestManaValueOnBattlefield(&Filter::NONLAND);

card!(
    index = index::CULLING_SCALES,
    oracle_id = "ec4f2266-3e8e-4194-8f06-d95ebc543661",
    scryfall_id = "db9a57d6-bba6-463d-ad12-6f93c035d16b",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Culling Scales",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::destroy(TargetSpec::Object(&NONLAND_LOWEST))],
        targets = Some(TargetReq::one(TargetSpec::Object(&NONLAND_LOWEST))),
    )],
);
