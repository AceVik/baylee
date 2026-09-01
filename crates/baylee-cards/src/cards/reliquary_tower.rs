//! Reliquary Tower — (no cost) — Land
//! Oracle: You have no maximum hand size.
//! Oracle: {T}: Add {C}.
//! Set: SOC #398 — Secrets of Strixhaven Commander | Scryfall ID: e2a27742-08c1-4153-af7f-25a7a98f585e | Oracle ID: c23e5b80-08d2-4e24-9908-fe2aa4f30f6f
// IMPLEMENTED — no-max-hand-size modifier + {C} mana.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind, StaticAbility,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(130),
    oracle_id: "c23e5b80-08d2-4e24-9908-fe2aa4f30f6f",
    scryfall_id: "e2a27742-08c1-4153-af7f-25a7a98f585e",
    faces: &[FaceDef {
        name: "Reliquary Tower",
        types: TypeSet::LAND,
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::Any,
            modifier: Modifier::NoMaxHandSize,
            cross_zone: false,
        }),
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::mana(ManaColor::Colorless, 1)],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
    ],
    ..CardDef::DEFAULT
};
