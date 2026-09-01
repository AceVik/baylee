//! Sea Gate Loremaster — {4}{U} — Creature — Merfolk Wizard Ally
//! Oracle: {T}: Draw a card for each Ally you control.
//! Set: ZEN #63 — Zendikar | Scryfall ID: 5cd723c8-4b3d-4fbb-a825-79934279382d | Oracle ID: 6eed122b-9760-47fd-8ba2-adeda8054e0d
// IMPLEMENTED — tap to draw per Ally.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, ZoneSel,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ALLY_YOU: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(141),
    oracle_id: "6eed122b-9760-47fd-8ba2-adeda8054e0d",
    scryfall_id: "5cd723c8-4b3d-4fbb-a825-79934279382d",
    faces: &[FaceDef {
        name: "Sea Gate Loremaster",
        mana_cost: baylee_core::mana!("{4}{U}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::MERFOLK, creature::WIZARD, creature::ALLY],
        power: Some(1),
        toughness: Some(3),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::DrawCards {
            amount: Amount::CountOf {
                filter: &ALLY_YOU,
                zone: ZoneSel::Battlefield,
            },
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};
