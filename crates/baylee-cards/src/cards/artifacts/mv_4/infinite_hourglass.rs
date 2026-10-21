//! Infinite Hourglass — {4} — Artifact
//! Oracle: At the beginning of your upkeep, put a time counter on this artifact.
//! Oracle: All creatures get +1/+0 for each time counter on this artifact.
//! Oracle: {3}: Remove a time counter from this artifact. Any player may activate this ability but only during any upkeep step.
//! Set: 5ED #378 — Fifth Edition | Scryfall ID: fbafc838-7458-4542-86f3-594326ec0691 | Oracle ID: 7c59ad65-ad1f-4dc4-bcf2-ea6b2b48fad8
// NOT SUPPORTED: all creatures getting +1/+0 per time counter on Infinite Hourglass is not supported, and only its controller (rather than any player) may activate the {3} ability during an upkeep step.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::INFINITE_HOURGLASS,
    oracle_id = "7c59ad65-ad1f-4dc4-bcf2-ea6b2b48fad8",
    scryfall_id = "fbafc838-7458-4542-86f3-594326ec0691",
    coverage = Coverage::Partial(
        "all creatures getting +1/+0 per time counter on Infinite Hourglass is not supported, and only its controller (rather than any player) may activate the {3} ability during an upkeep step",
    ),
    faces = &[face!(
        name = "Infinite Hourglass",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::AddCounter {
                kind: CounterKind::Time,
                amount: Amount::Fixed(1),
            }],
        ),
        activated!(
            cost!("{3}"),
            &[Effect::RemoveCounterSelf {
                kind: CounterKind::Time,
                n: 1,
            }],
            condition = Some(Condition::DuringStep(StepKind::Upkeep)),
        ),
    ],
);
