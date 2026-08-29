//! Chromatic Lantern — {3} — Artifact
//! Oracle: Lands you control have "{T}: Add one mana of any color."
//! Oracle: {T}: Add one mana of any color.
//! Set: MBC #73 — Mystery Booster Commander Edition | Scryfall ID: 9b29492a-8bdd-4806-8d1b-3058ed277cc1 | Oracle ID: 539f5396-d99a-417d-a84c-dff7930b5900
// IMPLEMENTED — its own any-color mana + the lands-you-control grant
// (GrantActivated static).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    ALL_MANA_COLORS, ANY_COLOR_MANA, AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef,
    CommanderRule, Cost, Coverage, Effect, FaceDef, Filter, KeywordSet, Layer, Modifier,
    PartnerKind, StaticAbility,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(19),
    oracle_id: "539f5396-d99a-417d-a84c-dff7930b5900",
    scryfall_id: "9b29492a-8bdd-4806-8d1b-3058ed277cc1",
    faces: &[FaceDef {
        name: "Chromatic Lantern",
        mana_cost: baylee_core::mana!("{3}"),
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
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: Filter::And(&[Filter::HasType(TypeSet::LAND), Filter::ControlledByYou]),
            modifier: Modifier::GrantActivated {
                cost: Cost::TAP,
                effects: ANY_COLOR_MANA,
                mana_ability: true,
            },
            cross_zone: false,
        }),
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddManaChoice {
                colors: ALL_MANA_COLORS,
                amount: Amount::Fixed(1),
                combination: false,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
