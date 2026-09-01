//! Sensei's Divining Top — {1} — Artifact
//! Oracle: {1}: Look at the top three cards of your library, then put them back in any order.
//! Oracle: {T}: Draw a card, then put Sensei's Divining Top on top of its owner's library.
//! Set: EMA #232 — Eternal Masters | Scryfall ID: e5142b7a-e580-4737-a4aa-2590f6610ceb | Oracle ID: 13575cf9-65c1-4861-b21e-eb2155e07766
// IMPLEMENTED — top-3 reorder + draw-and-replace.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost, Coverage,
    Effect, FaceDef, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(142),
    oracle_id: "13575cf9-65c1-4861-b21e-eb2155e07766",
    scryfall_id: "e5142b7a-e580-4737-a4aa-2590f6610ceb",
    faces: &[FaceDef {
        name: "Sensei's Divining Top",
        mana_cost: baylee_core::mana!("{1}"),
        types: TypeSet::ARTIFACT,
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Activated {
            cost: Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[],
            },
            effects: &[Effect::ReorderTopLibrary { count: 3 }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[
                Effect::DrawCards {
                    amount: Amount::Fixed(1),
                },
                Effect::PutSourceOnTopOfLibrary,
            ],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
    ..CardDef::DEFAULT
};
