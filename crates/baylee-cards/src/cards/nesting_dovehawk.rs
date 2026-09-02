//! Nesting Dovehawk — {2}{W} — Creature — Bird
//! Oracle: Flying
//! Oracle: At the beginning of combat on your turn, populate. (Create a token that's a copy of a creature token you control.)
//! Oracle: Whenever a creature token you control enters, put a +1/+1 counter on this creature.
//! Set: EOC #25 — Edge of Eternities Commander | Scryfall ID: c58ff93f-7135-40af-92ce-358da48694dc | Oracle ID: fe8fc442-ed17-40b2-8624-69f2eed3f9be
// IMPLEMENTED — populate (token-only copy) + token-ETB growth.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static CREATURE_TOKEN_YOU_CONTROL: Filter =
    Filter::And(&[Filter::IsToken, Filter::CREATURE, Filter::ControlledByYou]);

card! {
    index: 103,
    oracle_id: "fe8fc442-ed17-40b2-8624-69f2eed3f9be",
    scryfall_id: "c58ff93f-7135-40af-92ce-358da48694dc",
    faces: &[face! {
        name: "Nesting Dovehawk",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::BIRD],
        power: Some(2),
        toughness: Some(2),
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::FLYING,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::StepBegin {
                step: StepKind::CombatBegin,
                whose: PlayerRel::You,
            }, &[Effect::CreateTokenCopyOf {
                target: Some(TargetSpec::Object(&CREATURE_TOKEN_YOU_CONTROL)),
                kicked_bonus: 0,
            }], targets: Some(TargetReq {
                spec: TargetSpec::Object(&CREATURE_TOKEN_YOU_CONTROL),
                min: 0,
                max: 1,
                count_is_x: false,
            })),
        triggered!(Trigger::EntersBattlefield(&CREATURE_TOKEN_YOU_CONTROL), &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]),
    ],
}
