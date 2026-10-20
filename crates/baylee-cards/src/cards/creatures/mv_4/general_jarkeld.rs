//! General Jarkeld — {3}{W} — Legendary Creature — Human Soldier
//! Oracle: {T}: Choose two target blocked attacking creatures. If each of those creatures could be blocked by all creatures that the other is blocked by, each creature that's blocking exactly one of those attacking creatures stops blocking it and is blocking the other attacking creature. Activate only during the declare blockers step.
//! Set: ICE #27 — Ice Age | Scryfall ID: 6a4f5a28-0bd2-4cc4-b67f-324e89193caa | Oracle ID: 64df77f6-4f7e-442a-971c-8ad3f7d1c979
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKED_ATTACKING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::Not(&Filter::Unblocked),
]);

// NOT SUPPORTED: "each creature that's blocking exactly one of those attacking creatures stops blocking it and is blocking the other attacking creature" —
// Effect::SwapBlockers expects two blocking creatures rather than two blocked attacking creatures.
card!(
    index = index::GENERAL_JARKELD,
    oracle_id = "64df77f6-4f7e-442a-971c-8ad3f7d1c979",
    scryfall_id = "6a4f5a28-0bd2-4cc4-b67f-324e89193caa",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "swapping blockers between two targeted blocked attacking creatures not supported"
    ),
    faces = &[face!(
        name = "General Jarkeld",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::SwapBlockers],
        targets = Some(TargetReq::exactly(
            TargetSpec::Object(&BLOCKED_ATTACKING_CREATURE),
            2
        )),
        condition = Some(Condition::DuringStep(StepKind::DeclareBlockers))
    )],
);
