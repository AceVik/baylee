//! Isolated Chapel — (no cost) — Land
//! Oracle: Isolated Chapel enters the battlefield tapped unless you control a PLAINS or an SWAMP.
//! {T}: Add White or Black.
//! Set: XLN #253 — Ixalan | Scryfall ID: 78814c92-b52c-462a-866f-3e7da9db9f70 | Oracle ID: 7e5d9efe-48a9-434b-bb09-056e0e09cc9a
// IMPLEMENTED — checkland (ETB tapped unless you control a PLAINS/SWAMP) + 2-color mana.
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
        Filter::HasSubtype(land::SWAMP),
    ]),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(75),
    oracle_id: "7e5d9efe-48a9-434b-bb09-056e0e09cc9a",
    scryfall_id: "78814c92-b52c-462a-866f-3e7da9db9f70",
    faces: &[FaceDef {
        name: "Isolated Chapel",
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
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaChoice {
            colors: &[ManaColor::White, ManaColor::Black],
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
