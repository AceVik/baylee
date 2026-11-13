//! Blood Frenzy — {1}{R} — Instant
//! Oracle: Cast this spell only before the combat damage step.
//! Oracle: Target attacking or blocking creature gets +4/+0 until end of turn. Destroy that creature at the beginning of the next end step.
//! Set: TMP #164 — Tempest | Scryfall ID: 06f39d83-e1ea-45c9-8181-2a2b6e5148da | Oracle ID: 8d3d5e87-8b12-4549-a46e-70a103dbcf44
// IMPLEMENTED — Cast only before the combat damage step; target attacking or blocking creature gets +4/+0 until end of turn and is destroyed at the beginning of the next end step.

use baylee_cards_dsl::prelude::*;

const ATTACKING_OR_BLOCKING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[Filter::Attacking, Filter::Blocking]),
]);

card!(
    index = index::BLOOD_FRENZY,
    oracle_id = "8d3d5e87-8b12-4549-a46e-70a103dbcf44",
    scryfall_id = "06f39d83-e1ea-45c9-8181-2a2b6e5148da",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blood Frenzy",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::PumpTarget {
                power: Amount::Fixed(4),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::destroy(TargetSpec::EventObject)],
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &ATTACKING_OR_BLOCKING_CREATURE
        ))),
        condition = Some(Condition::BeforeStep(StepKind::CombatDamage)),
    )],
);
