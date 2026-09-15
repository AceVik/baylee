//! Nesting Dovehawk — {3}{W} — Creature — Bird
//! Oracle: Flying
//! Oracle: At the beginning of combat on your turn, populate. (Create a token that's a copy of a creature token you control.)
//! Oracle: Whenever a creature token you control enters, put a +1/+1 counter on this creature.
//! Set: MOC #17 — March of the Machine Commander | Scryfall ID: c58ff93f-7135-40af-92ce-358da48694dc | Oracle ID: fe8fc442-ed17-40b2-8624-69f2eed3f9be
// IMPLEMENTED — populate (token-only copy) + token-ETB growth.
// NOT SUPPORTED: populate is a *choice*, not a target (CR 701.36), and the
// DSL has only targets to say it with. `min: 1` is the closer of the two
// approximations: populate is mandatory when you control a creature token,
// so a player must not be able to answer the empty list, and a Dovehawk with
// no token to copy is better removed from the stack (the target path's
// CR 603.3d) than left there resolving into nothing. What the approximation
// still costs is a creature token of your own with shroud, which populate
// may copy and a target may not.

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
        mana_cost: mana!("{3}{W}"),
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
                min: 1,
                max: 1,
                count_is_x: false,
            })),
        triggered!(Trigger::EntersBattlefield(&CREATURE_TOKEN_YOU_CONTROL), &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]),
    ],
}
