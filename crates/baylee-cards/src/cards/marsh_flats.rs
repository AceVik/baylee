//! Marsh Flats — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Plains or Swamp card, put it onto the battlefield, then shuffle.
//! Set: MH2 #248 — Modern Horizons 2 | Scryfall ID: 9db3ba6d-eb7f-4f5b-9a3b-c6239c3baa42 | Oracle ID: dab520d0-20b4-4273-ba6b-eb07f85ea433
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Plains/Swamp
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
    Filter::HasSubtype(land::SWAMP),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(91),
    oracle_id: "dab520d0-20b4-4273-ba6b-eb07f85ea433",
    scryfall_id: "9db3ba6d-eb7f-4f5b-9a3b-c6239c3baa42",
    faces: &[FaceDef {
        name: "Marsh Flats",
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
            tapped: true,
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
