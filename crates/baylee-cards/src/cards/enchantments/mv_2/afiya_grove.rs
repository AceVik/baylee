//! Afiya Grove — {1}{G} — Enchantment
//! Oracle: This enchantment enters with three +1/+1 counters on it.
//! Oracle: At the beginning of your upkeep, move a +1/+1 counter from this enchantment onto target creature.
//! Oracle: When this enchantment has no +1/+1 counters on it, sacrifice it.
//! Set: MIR #205 — Mirage | Scryfall ID: c495e1f3-0845-4556-83e6-de8b9d518d1d | Oracle ID: 056a98aa-f3f7-4ac9-9755-3e8a60a59abb
// NOT SUPPORTED: state-triggered sacrifice when it has no +1/+1 counters on it is approximated via Trigger::LastCounterRemoved when you remove the last +1/+1 counter.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AFIYA_GROVE,
    oracle_id = "056a98aa-f3f7-4ac9-9755-3e8a60a59abb",
    scryfall_id = "c495e1f3-0845-4556-83e6-de8b9d518d1d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "enters with three +1/+1 counters, moves a +1/+1 counter onto target creature at the beginning of your upkeep, and sacrifices itself when you remove the last +1/+1 counter from it; sacrificing as a state-triggered ability whenever it has no +1/+1 counters on it is not supported",
    ),
    faces = &[face!(
        name = "Afiya Grove",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(3),
        }],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MoveCounter {
                from: TargetSlot::This,
                to: TargetSlot::First,
                kind: Some(CounterKind::P1P1),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        triggered!(
            Trigger::LastCounterRemoved {
                kind: CounterKind::P1P1,
            },
            &[Effect::SacrificeSelf],
        ),
    ],
);
