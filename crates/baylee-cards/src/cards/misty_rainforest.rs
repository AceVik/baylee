//! Misty Rainforest — (no cost) — Land
//! Oracle: {T}, Pay 1 life, Sacrifice this land: Search your library for a Forest or Island card, put it onto the battlefield, then shuffle.
//! Set: MH2 #250 — Modern Horizons 2 | Scryfall ID: 88231c0d-0cc8-44ec-bf95-81d1710ac141 | Oracle ID: 09dd85aa-47bc-4713-a9b9-8b52ff2285ed
// IMPLEMENTED — fetchland (tap + pay life + sacrifice → search Forest/Island
// to the battlefield tapped, shuffle).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Effect, FaceDef, Filter, Find, KeywordSet, PartnerKind, SearchDest,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, land};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static SEARCH_FILTER: Filter = Filter::Or(&[
    Filter::HasSubtype(land::FOREST),
    Filter::HasSubtype(land::ISLAND),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(97),
    oracle_id: "09dd85aa-47bc-4713-a9b9-8b52ff2285ed",
    scryfall_id: "88231c0d-0cc8-44ec-bf95-81d1710ac141",
    faces: &[FaceDef {
        name: "Misty Rainforest",
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
            finds: &[Find::BATTLEFIELD_TAPPED],
            optional: false,
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
    }],
    ..CardDef::DEFAULT
};

// Fetchland family coverage lives in baylee-engine (fetchland test with
// Polluted Delta + the land-wave group test).
