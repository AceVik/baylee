//! Electrostatic Bolt — {R} — Instant
//! Oracle: Electrostatic Bolt deals 2 damage to target creature. If it's an artifact creature, Electrostatic Bolt deals 4 damage to it instead.
//! Set: MRD #89 — Mirrodin | Scryfall ID: c455c8d4-6f20-4dcb-8e82-c2bb70d6bc3e | Oracle ID: 6fc26faa-94a8-433f-8863-126c2d2e73b0
use baylee_cards_dsl::prelude::*;

static ARTIFACT_CREATURE: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CREATURE]);

card!(
    index = index::ELECTROSTATIC_BOLT,
    oracle_id = "6fc26faa-94a8-433f-8863-126c2d2e73b0",
    scryfall_id = "c455c8d4-6f20-4dcb-8e82-c2bb70d6bc3e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Electrostatic Bolt",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::IfTargetMatchesElse {
            filter: &ARTIFACT_CREATURE,
            then: &[Effect::DealDamage {
                amount: Amount::Fixed(4),
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            otherwise: &[Effect::DealDamage {
                amount: Amount::Fixed(2),
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
