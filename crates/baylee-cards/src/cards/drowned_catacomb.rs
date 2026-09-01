//! Drowned Catacomb — (no cost) — Land
//! Oracle: Drowned Catacomb enters the battlefield tapped unless you control a ISLAND or an SWAMP.
//! {T}: Add Blue or Black.
//! Set: XLN #252 — Ixalan | Scryfall ID: ebea49ab-e5cf-46d9-ae35-226a7321ede0 | Oracle ID: 819fc966-434e-470f-91e9-a38df974ad17
// IMPLEMENTED — checkland (ETB tapped unless you control a ISLAND/SWAMP) + 2-color mana.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, EnterModifier, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static CHECK: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasType(TypeSet::LAND),
    Filter::Or(&[
        Filter::HasSubtype(land::ISLAND),
        Filter::HasSubtype(land::SWAMP),
    ]),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(36),
    oracle_id: "819fc966-434e-470f-91e9-a38df974ad17",
    scryfall_id: "ebea49ab-e5cf-46d9-ae35-226a7321ede0",
    faces: &[FaceDef {
        name: "Drowned Catacomb",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: &[ManaColor::Blue, ManaColor::Black],
            amount: Amount::Fixed(1),
            combination: false,
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};
