//! Bloodstained Mire — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Swamp or Mountain card, put it onto the battlefield, then shuffle.
//! Set: MH3 #216 — Modern Horizons 3 | Scryfall ID: 579743fe-f71e-4cb2-8629-d6b02ed1591d | Oracle ID: fc0707c7-d504-4ccf-a0d2-3eb6e26e7a57
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Swamp/Mountain
// to the battlefield tapped, shuffle).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, SearchDest,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static SEARCH_FILTER: Filter = Filter::Or(&[
    Filter::HasSubtype(land::SWAMP),
    Filter::HasSubtype(land::MOUNTAIN),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(13),
    oracle_id: "fc0707c7-d504-4ccf-a0d2-3eb6e26e7a57",
    scryfall_id: "579743fe-f71e-4cb2-8629-d6b02ed1591d",
    faces: &[FaceDef {
        name: "Bloodstained Mire",
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
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost {
            mana: ManaCost::ZERO,
            parts: &[
                CostPart::TapSelf,
                CostPart::SacrificeSelf,
                CostPart::PayLife(1),
            ],
        },
        effects: &[Effect::SearchLibrary {
            filter: &SEARCH_FILTER,
            dest: SearchDest::Battlefield,
            tapped: false,
            shuffle: true,
            optional: false,
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
    }],
};

#[cfg(test)]
mod tests {
    // Fetchland family coverage lives in baylee-engine (fetchland test with
    // Polluted Delta + the land-wave group test).
}
