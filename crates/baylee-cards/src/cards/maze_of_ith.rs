//! Maze of Ith — (no cost) — Land
//! Oracle: {T}: Untap target attacking creature. Prevent all combat damage that would be dealt to and dealt by that creature this turn.
//! Set: DMR #250 — Dominaria Remastered | Scryfall ID: 5889fde1-730d-43d0-aaa4-499784a80530 | Oracle ID: 38a12bd7-4394-44a8-91a0-6a4ff7fa4f71
// IMPLEMENTED — untap + damage prevention to/from the target until EOT.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Duration,
    Effect, FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ATTACKING_CREATURE: Filter =
    Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::Attacking]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(93),
    oracle_id: "38a12bd7-4394-44a8-91a0-6a4ff7fa4f71",
    scryfall_id: "5889fde1-730d-43d0-aaa4-499784a80530",
    faces: &[FaceDef {
        name: "Maze of Ith",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
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
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[
            Effect::UntapTarget,
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::This,
                modifier: Modifier::PreventDamageToIt,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::This,
                modifier: Modifier::PreventDamageFromIt,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        target: Some(TargetSpec::Object(&ATTACKING_CREATURE)),
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
    }],
};

#[cfg(test)]
mod tests {}
