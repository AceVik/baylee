//! Riptide Laboratory — (no cost) — Land
//! Oracle: {T}: Add {C}. {T}: Return target Wizard to its owner\u{2019}s hand.
//! Set: C14 #305 — Commander 2014 | Scryfall ID: 25a9cb87-e572-4885-8561-1d4b158ec7e4 | Oracle ID: 444d50dd-a44a-42db-bbf6-d0978e3bd6a3
// IMPLEMENTED.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, PlayerRel, TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static WIZARD: Filter = Filter::HasSubtype(creature::WIZARD);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(134),
    oracle_id: "444d50dd-a44a-42db-bbf6-d0978e3bd6a3",
    scryfall_id: "25a9cb87-e572-4885-8561-1d4b158ec7e4",
    faces: &[FaceDef {
        name: "Riptide Laboratory",
        types: TypeSet::LAND,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::Colorless,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::ReturnToHand {
                target: TargetSpec::Object(&WIZARD),
            }],
            target: Some(TargetSpec::Object(&WIZARD)),
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
    ..CardDef::DEFAULT
};
