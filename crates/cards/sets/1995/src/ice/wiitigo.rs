//! Wiitigo — {3}{G}{G}{G} — Creature — Yeti
//! Oracle: This creature enters with six +1/+1 counters on it.
//! Oracle: At the beginning of your upkeep, put a +1/+1 counter on this creature if it has blocked or been blocked since your last upkeep. Otherwise, remove a +1/+1 counter from it.
//! Set: ME2 #186 — Masters Edition II | Scryfall ID: ee3c3de0-5568-4113-8481-7203fa7faef1 | Oracle ID: 7f956a21-854e-41cc-96f9-ad774e69fa8e
// NOT SUPPORTED: checking whether this creature blocked or was blocked since your last upkeep is not supported; enters with six +1/+1 counters and always removes a +1/+1 counter at the beginning of your upkeep.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WIITIGO,
    oracle_id = "7f956a21-854e-41cc-96f9-ad774e69fa8e",
    scryfall_id = "ee3c3de0-5568-4113-8481-7203fa7faef1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "checking whether this creature blocked or was blocked since your last upkeep is not supported; enters with six +1/+1 counters and always removes a +1/+1 counter at the beginning of your upkeep",
    ),
    faces = &[face!(
        name = "Wiitigo",
        mana_cost = mana!("{3}{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::YETI],
        power = Some(0),
        toughness = Some(0),
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(6),
        }],
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
        &[Effect::RemoveCounterSelf {
            kind: CounterKind::P1P1,
            n: 1,
        }]
    )],
);
