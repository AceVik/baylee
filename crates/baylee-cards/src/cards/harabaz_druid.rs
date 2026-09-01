//! Harabaz Druid — {1}{G} — Creature — Human Druid Ally
//! Oracle: {T}: Add X mana of any one color, where X is the number of Allies you control.
//! Set: WWK #105 — Worldwake | Scryfall ID: 78a538cf-2291-49aa-8429-17d97d454479 | Oracle ID: ead985ec-f29f-4a3b-b8b1-061142cc5bd1
// IMPLEMENTED — dynamic Ally mana (choose a color, X = Allies).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, ZoneSel,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static ALLIES_YOU: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(66),
    oracle_id: "ead985ec-f29f-4a3b-b8b1-061142cc5bd1",
    scryfall_id: "78a538cf-2291-49aa-8429-17d97d454479",
    faces: &[FaceDef {
        name: "Harabaz Druid",
        mana_cost: baylee_core::mana!("{1}{G}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::HUMAN, creature::DRUID, creature::ALLY],
        power: Some(0),
        toughness: Some(1),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::mana_combination(
            &[
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ],
            Amount::CountOf {
                filter: &ALLIES_YOU,
                zone: ZoneSel::Battlefield,
            },
        )],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};

// X = Allies is delivered by the mana effect.s dynamic Amount::CountOf
// (evaluated at resolution against your battlefield).
