//! Teleport — {U}{U}{U} — Instant
//! Oracle: Cast this spell only during the declare attackers step.
//! Oracle: Target creature can't be blocked this turn.
//! Set: CHR #26 — Chronicles | Scryfall ID: 39c65d23-4bc5-4f79-a0e5-0cd4b2660f96 | Oracle ID: 9efcf3d1-a0be-4a34-90c6-85a46b48fb83
// IMPLEMENTED — the cast is gated to the declare attackers step and the
// target gains KeywordSet::UNBLOCKABLE until end of turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TELEPORT,
    oracle_id = "9efcf3d1-a0be-4a34-90c6-85a46b48fb83",
    scryfall_id = "39c65d23-4bc5-4f79-a0e5-0cd4b2660f96",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Teleport",
        mana_cost = mana!("{U}{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::UNBLOCKABLE,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        condition = Some(Condition::DuringStep(StepKind::DeclareAttackers))
    )],
);
