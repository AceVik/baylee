//! Afiya Grove — {1}{G} — Enchantment
//! Oracle: This enchantment enters with three +1/+1 counters on it.
//! Oracle: At the beginning of your upkeep, move a +1/+1 counter from this enchantment onto target creature.
//! Oracle: When this enchantment has no +1/+1 counters on it, sacrifice it.
//! Set: MIR #205 — Mirage | Scryfall ID: c495e1f3-0845-4556-83e6-de8b9d518d1d | Oracle ID: 056a98aa-f3f7-4ac9-9755-3e8a60a59abb
// IMPLEMENTED — enters with three +1/+1 counters; at your upkeep moves one onto target creature; a state trigger sacrifices it when it has none.

use baylee_cards_dsl::prelude::*;

static NO_P1P1_COUNTERS: Condition = Condition::CountersOnSelfExactly(CounterKind::P1P1, 0);

card!(
    index = index::AFIYA_GROVE,
    oracle_id = "056a98aa-f3f7-4ac9-9755-3e8a60a59abb",
    scryfall_id = "c495e1f3-0845-4556-83e6-de8b9d518d1d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
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
        triggered!(Trigger::State(&NO_P1P1_COUNTERS), &[Effect::SacrificeSelf],),
    ],
);
