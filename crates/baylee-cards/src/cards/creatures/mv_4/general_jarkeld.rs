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

// NOT SUPPORTED: "If each of those creatures could be blocked by all creatures that the other is
// blocked by, each creature that's blocking exactly one of those attacking creatures stops
// blocking it and is blocking the other attacking creature." No effect moves blockers between
// two attackers. `Effect::SwapBlockers` is Sorrow's Path's exchange of two *blocking*
// creatures: given two attackers as its targets it finds nothing either of them blocks and
// changes nothing. So the ability is offered and paid as printed (tap, two target blocked
// attacking creatures, declare blockers step only) and resolves to no change at all.
card!(
    index = index::GENERAL_JARKELD,
    oracle_id = "64df77f6-4f7e-442a-971c-8ad3f7d1c979",
    scryfall_id = "6a4f5a28-0bd2-4cc4-b67f-324e89193caa",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "the reblock is not played: the ability taps and targets two blocked attackers, and no blocker moves"
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
