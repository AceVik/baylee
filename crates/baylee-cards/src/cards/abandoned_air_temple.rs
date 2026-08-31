//! Abandoned Air Temple — (no cost) — Land
//! Oracle: This land enters tapped unless you control a basic land.
//! {T}: Add {W}.
//! {3}{W}, {T}: Put a +1/+1 counter on each creature you control.
//! Set: TLA #260 — Avatar: The Last Airbender | Scryfall ID: 9c0433f9-8f1e-4a19-a83f-a41925f1b1a9 | Oracle ID: 9575d7ce-f26d-4b90-87a3-6329e9799572
// IMPLEMENTED — checkland variant + team pump activation.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, CostPart,
    CounterKind, Coverage, Effect, EnterModifier, FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static BASIC_LAND_YOU: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::HasType(TypeSet::LAND),
    Filter::HasSupertype(SupertypeSet::BASIC),
]);
static YOUR_CREATURES: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasType(TypeSet::CREATURE)]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(1),
    oracle_id: "9575d7ce-f26d-4b90-87a3-6329e9799572",
    scryfall_id: "9c0433f9-8f1e-4a19-a83f-a41925f1b1a9",
    faces: &[FaceDef {
        name: "Abandoned Air Temple",
        types: TypeSet::LAND,
        enter_modifiers: &[EnterModifier::TappedUnless(&BASIC_LAND_YOU)],
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::White,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::Activated {
            cost: Cost {
                mana: baylee_core::mana!("{3}{W}"),
                parts: &[CostPart::TapSelf],
            },
            effects: &[Effect::AddCounterFilter {
                filter: &YOUR_CREATURES,
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
