//! Scalding Tarn — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Island or Mountain card, put it onto the battlefield, then shuffle.
//! Set: MH2 #254 — Modern Horizons 2 | Scryfall ID: 71e491c5-8c07-449b-b2f1-ffa052e6d311 | Oracle ID: cb027150-848c-4a66-88ad-e20222304dd8
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Island/Mountain
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
    Filter::HasSubtype(land::ISLAND),
    Filter::HasSubtype(land::MOUNTAIN),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(139),
    oracle_id: "cb027150-848c-4a66-88ad-e20222304dd8",
    scryfall_id: "71e491c5-8c07-449b-b2f1-ffa052e6d311",
    faces: &[FaceDef {
        name: "Scalding Tarn",
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
            tapped: true,
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
