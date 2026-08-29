//! Liquimetal Torque — {2} — Artifact
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Target nonland permanent becomes an artifact in addition to its other types until end of turn.
//! Set: MH2 #228 — Modern Horizons 2 | Scryfall ID: 13c6101a-da40-4785-8ccb-4e779bbbdb55 | Oracle ID: b7d4b7dd-fbb1-4ca3-875f-ef13a95e66ad
// IMPLEMENTED — mana rock + timed type change.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Duration,
    Effect, FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static NONLAND: Filter = Filter::LacksType(TypeSet::LAND);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(86),
    oracle_id: "b7d4b7dd-fbb1-4ca3-875f-ef13a95e66ad",
    scryfall_id: "13c6101a-da40-4785-8ccb-4e779bbbdb55",
    faces: &[FaceDef {
        name: "Liquimetal Torque",
        mana_cost: baylee_core::mana!("{2}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
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
            effects: &[Effect::CreateContinuousEffect {
                layer: Layer::Type,
                filter: &NONLAND,
                modifier: Modifier::AddType(TypeSet::ARTIFACT),
                duration: Duration::UntilEndOfTurn,
            }],
            target: Some(TargetSpec::Object(&NONLAND)),
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
