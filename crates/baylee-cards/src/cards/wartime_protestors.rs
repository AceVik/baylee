//! Wartime Protestors — {2}{R} — Creature — Human Rebel Ally
//! Oracle: Haste
//! Oracle: Whenever another Ally you control enters, put a +1/+1 counter on that creature and it gains haste until end of turn.
//! Set: TLA #192 — Avatar: The Last Airbender | Scryfall ID: bac81940-d717-49ff-83b2-16a22bb2c988 | Oracle ID: 6557813b-4ee7-4881-a37c-10c8ea097360
// IMPLEMENTED — haste + rally counter + temporary haste on entering Allies
// (event-object targeting).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CounterKind, Coverage, Duration, Effect, FaceDef,
    Filter, KeywordSet, Layer, Modifier, PartnerKind, TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static OTHER_ALLY: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasSubtype(creature::ALLY),
    Filter::Another,
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(187),
    oracle_id: "6557813b-4ee7-4881-a37c-10c8ea097360",
    scryfall_id: "bac81940-d717-49ff-83b2-16a22bb2c988",
    faces: &[FaceDef {
        name: "Wartime Protestors",
        mana_cost: baylee_core::mana!("{2}{R}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::REBEL, creature::ALLY],
        power: Some(3),
        toughness: Some(2),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Red]),
    keywords: KeywordSet::HASTE,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&OTHER_ALLY),
        once_per_turn: false,
        effects: &[
            Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Ability,
                filter: &Filter::This,
                modifier: Modifier::AddKeyword(KeywordSet::HASTE),
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets: Some(TargetReq::one(TargetSpec::EventObject)),
    }],
    ..CardDef::DEFAULT
};
