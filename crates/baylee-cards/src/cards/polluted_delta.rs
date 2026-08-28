//! Polluted Delta — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for an Island or Swamp card, put it onto the battlefield, then shuffle.
//! Set: MKC #246 — Commander: Murders at Karlov Manor | Scryfall ID: 6e288374-2b71-4ace-b1d2-a19fee6cb4af | Oracle ID: ef86989d-ce80-4e55-aece-7d11710eeffa
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Island/Swamp
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
    Filter::HasSubtype(land::SWAMP),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(116),
    oracle_id: "ef86989d-ce80-4e55-aece-7d11710eeffa",
    scryfall_id: "6e288374-2b71-4ace-b1d2-a19fee6cb4af",
    faces: &[FaceDef {
        name: "Polluted Delta",
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
    // Engine-level test lives in baylee-engine (fetchland_works): activation
    // pays tap+sacrifice+1 life, offers only Island/Swamp options, puts the
    // chosen card onto the battlefield tapped, and shuffles.
}
