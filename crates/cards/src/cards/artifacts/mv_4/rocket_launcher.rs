//! Rocket Launcher — {4} — Artifact
//! Oracle: {2}: This artifact deals 1 damage to any target. Destroy this artifact at the beginning of the next end step. Activate only if you've controlled this artifact continuously since the beginning of your most recent turn.
//! Set: SUM #272 — Summer Magic / Edgar | Scryfall ID: 551c2217-fdf5-465f-8522-1b88c111be7d | Oracle ID: 11720db4-5b6b-49ba-bf31-4d944921d6f1
// IMPLEMENTED — {2} deals 1 damage to any target and schedules its own destruction at the next end step; active only if controlled since your most recent turn began.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ROCKET_LAUNCHER,
    oracle_id = "11720db4-5b6b-49ba-bf31-4d944921d6f1",
    scryfall_id = "551c2217-fdf5-465f-8522-1b88c111be7d",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rocket Launcher",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}"),
        &[
            Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::destroy(TargetSpec::ThisObject)],
            },
        ],
        target = Some(TargetSpec::AnyTarget),
        condition = Some(Condition::SourceMatches(&Filter::ControlledSinceTurnBegan)),
    )],
);
