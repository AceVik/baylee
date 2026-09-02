//! Soulherder — {1}{W}{U} — Creature — Spirit
//! Oracle: Whenever a creature is exiled from the battlefield, put a +1/+1 counter on this creature.
//! Oracle: At the beginning of your end step, you may exile another target creature you control, then return that card to the battlefield under its owner's control.
//! Set: KHC #93 — Kaldheim Commander | Scryfall ID: 50bc0f5b-7421-45b9-af85-86dd9821b7d8 | Oracle ID: 92019547-f6db-4ea6-8356-d0a90ace5662
// IMPLEMENTED — exile-watcher growth + end-step blink.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static ANOTHER_CREATURE_YOU_CONTROL: Filter =
    Filter::And(&[Filter::Another, Filter::CREATURE, Filter::ControlledByYou]);

card! {
    index: 153,
    oracle_id: "92019547-f6db-4ea6-8356-d0a90ace5662",
    scryfall_id: "50bc0f5b-7421-45b9-af85-86dd9821b7d8",
    faces: &[face! {
        name: "Soulherder",
        mana_cost: baylee_core::mana!("{1}{W}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::SPIRIT],
        power: Some(1),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::ExiledFromBattlefield(&Filter::CREATURE), &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]),
        triggered!(Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            }, &[Effect::Blink {
                target: TargetSpec::Object(&ANOTHER_CREATURE_YOU_CONTROL),
            }], targets: Some(TargetReq {
                spec: TargetSpec::Object(&ANOTHER_CREATURE_YOU_CONTROL),
                min: 0,
                max: 1,
                count_is_x: false,
            })),
    ],
}
