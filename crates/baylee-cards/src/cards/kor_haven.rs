//! Kor Haven — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}{W}, {T}: Prevent all combat damage that would be dealt by target attacking creature this turn.
//! Set: NEM #141 — Nemesis | Scryfall ID: 3d5529ca-5c20-4dfd-8595-96d6dfa6debe | Oracle ID: 276cece9-f9f2-46e6-ae76-daddaa2fb9ab
// IMPLEMENTED — {C} mana + attacking-creature damage prevention.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Duration, Effect, FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static ATTACKING_CREATURE: Filter =
    Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::Attacking]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(84),
    oracle_id: "276cece9-f9f2-46e6-ae76-daddaa2fb9ab",
    scryfall_id: "3d5529ca-5c20-4dfd-8595-96d6dfa6debe",
    faces: &[FaceDef {
        name: "Kor Haven",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::LEGENDARY,
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
        miracle: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
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
            cost: Cost {
                mana: baylee_core::mana!("{1}{W}"),
                parts: &[CostPart::TapSelf],
            },
            effects: &[Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::This,
                modifier: Modifier::PreventDamageFromIt,
                duration: Duration::UntilEndOfTurn,
            }],
            target: Some(TargetSpec::Object(&ATTACKING_CREATURE)),
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
