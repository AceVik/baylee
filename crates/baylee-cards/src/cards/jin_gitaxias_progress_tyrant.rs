//! Jin-Gitaxias, Progress Tyrant — {5}{U}{U} — Legendary Creature — Phyrexian Praetor
//! Oracle: Whenever you cast an artifact, instant, or sorcery spell, copy that spell. You may choose new targets for the copy. This ability triggers only once each turn. (A copy of a permanent spell becomes a token.)
//! Oracle: Whenever an opponent casts an artifact, instant, or sorcery spell, counter that spell. This ability triggers only once each turn.
//! Set: NEO #59 — Kamigawa: Neon Dynasty | Scryfall ID: c57b4876-5387-4f73-b8e2-8e7bdca8b0bc | Oracle ID: f5daadc1-98ff-480a-82bb-fe7bfaa7b60e
// IMPLEMENTED — once-per-turn spell copy + once-per-turn counter.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static YOUR_AIS_SPELL: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[
        Filter::ARTIFACT,
        Filter::HasType(TypeSet::INSTANT),
        Filter::HasType(TypeSet::SORCERY),
    ]),
]);
static OPPONENT_AIS_SPELL: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::Or(&[
        Filter::ARTIFACT,
        Filter::HasType(TypeSet::INSTANT),
        Filter::HasType(TypeSet::SORCERY),
    ]),
]);

card! {
    index: 78,
    oracle_id: "f5daadc1-98ff-480a-82bb-fe7bfaa7b60e",
    scryfall_id: "c57b4876-5387-4f73-b8e2-8e7bdca8b0bc",
    faces: &[face! {
        name: "Jin-Gitaxias, Progress Tyrant",
        mana_cost: baylee_core::mana!("{5}{U}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::PHYREXIAN, creature::PRAETOR],
        power: Some(5),
        toughness: Some(5),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::SpellCast(&YOUR_AIS_SPELL), &[Effect::CopyTargetSpell { mods: &[] }], once_per_turn: true, targets: Some(TargetReq::one(
                TargetSpec::EventObject,
            ))),
        triggered!(Trigger::SpellCast(&OPPONENT_AIS_SPELL), &[Effect::CounterTargetSpell], once_per_turn: true, targets: Some(TargetReq::one(
                TargetSpec::EventObject,
            ))),
    ],
}
