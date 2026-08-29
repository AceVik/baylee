//! Glacial Fortress — (no cost) — Land
//! Oracle: Glacial Fortress enters the battlefield tapped unless you control a PLAINS or an ISLAND.
//! {T}: Add White or Blue.
//! Set: XLN #251 — Ixalan | Scryfall ID: d673a2d5-0c61-48dc-8c8d-06f0c7b6b8bf | Oracle ID: 027dd013-baa7-4111-b3c9-f4d1414e9c45
// IMPLEMENTED — checkland (ETB tapped unless you control a PLAINS/ISLAND) + 2-color mana.
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
        Filter::HasSubtype(land::PLAINS),
        Filter::HasSubtype(land::ISLAND),
    ]),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(59),
    oracle_id: "027dd013-baa7-4111-b3c9-f4d1414e9c45",
    scryfall_id: "d673a2d5-0c61-48dc-8c8d-06f0c7b6b8bf",
    faces: &[FaceDef {
        name: "Glacial Fortress",
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
        enter_modifiers: &[EnterModifier::TappedUnless(&CHECK)],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: &[ManaColor::White, ManaColor::Blue],
            amount: Amount::Fixed(1),
            combination: false,
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
};

#[cfg(test)]
mod tests {}
