//! Exotic Orchard — (no cost) — Land
//! Oracle: {T}: Add one mana of any color that a land an opponent controls could produce.
//! Set: MBC #79 — Mystery Booster Commander Edition | Scryfall ID: d11c5fe0-1528-4c94-a8cc-42bcab9d7487 | Oracle ID: 27b047e3-0d41-45e2-98e9-9391d7923a1e
// IMPLEMENTED — color choice from opponents' lands' producible mana
// (precomputed on the lands' characteristics at creation).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    FaceDef, Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(48),
    oracle_id: "27b047e3-0d41-45e2-98e9-9391d7923a1e",
    scryfall_id: "d11c5fe0-1528-4c94-a8cc-42bcab9d7487",
    faces: &[FaceDef {
        name: "Exotic Orchard",
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
        miracle: None,
        delve: false,
        convoke: false,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddManaLandColor { mine: false }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
    }],
};

#[cfg(test)]
mod tests {}
