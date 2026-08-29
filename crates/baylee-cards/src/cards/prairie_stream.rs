//! Prairie Stream — (no cost) — Land
//! Oracle: Prairie Stream enters the battlefield tapped unless you control a PLAINS or an ISLAND.
//! {T}: Add White or Blue.
//! Set: BFZ #241 — Battle for Zendikar | Scryfall ID: b2e133b4-2263-4ac2-8d16-7bf307d5e104 | Oracle ID: 5330e24a-8568-446e-840a-594cd08bd1bc
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
    index: CardIndex::new(117),
    oracle_id: "5330e24a-8568-446e-840a-594cd08bd1bc",
    scryfall_id: "b2e133b4-2263-4ac2-8d16-7bf307d5e104",
    faces: &[FaceDef {
        name: "Prairie Stream",
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
