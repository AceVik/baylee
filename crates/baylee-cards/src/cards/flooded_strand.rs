//! Flooded Strand — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Plains or Island card, put it onto the battlefield, then shuffle.
//! Set: MH3 #220 — Modern Horizons 3 | Scryfall ID: 8f85e12c-196b-4459-b81f-0c9c854e9f57 | Oracle ID: f3c7af78-a77d-4134-82a2-a5ce84285a84
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Plains/Island
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
    Filter::HasSubtype(land::PLAINS),
    Filter::HasSubtype(land::ISLAND),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(52),
    oracle_id: "f3c7af78-a77d-4134-82a2-a5ce84285a84",
    scryfall_id: "8f85e12c-196b-4459-b81f-0c9c854e9f57",
    faces: &[FaceDef {
        name: "Flooded Strand",
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
