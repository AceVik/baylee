//! Arid Mesa — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Mountain or Plains card, put it onto the battlefield, then shuffle.
//! Set: MH2 #244 — Modern Horizons 2 | Scryfall ID: 25ac5405-df7b-4097-914a-022cb18e20d4 | Oracle ID: c5acf2a5-40f4-433d-a74d-1cb56c521464
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Mountain/Plains
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
    Filter::HasSubtype(land::MOUNTAIN),
    Filter::HasSubtype(land::PLAINS),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(7),
    oracle_id: "c5acf2a5-40f4-433d-a74d-1cb56c521464",
    scryfall_id: "25ac5405-df7b-4097-914a-022cb18e20d4",
    faces: &[FaceDef {
        name: "Arid Mesa",
        types: TypeSet::LAND,
        ..FaceDef::DEFAULT
    }],
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
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {
    // Fetchland family coverage lives in baylee-engine (fetchland test with
    // Polluted Delta + the land-wave group test).
}
