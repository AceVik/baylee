//! Maskwood Nexus — {4} — Artifact
//! Oracle: Creatures you control are every creature type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.
//! {3}, {T}: Create a 2/2 blue Shapeshifter creature token with changeling. (It is every creature type.)
//! Set: KHM #240 — Kaldheim | Scryfall ID: 1246c42d-57c0-4cba-959a-15ad89d8a50b | Oracle ID: 9b2cdbed-c733-409b-b0e4-2c8960c25111
// IMPLEMENTED — cross-zone type granting (library/hand/graveyard/stack) +
// shapeshifter token creation.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Effect, FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind, StaticAbility, TokenDef,
    ZoneRef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

// Creatures you control (battlefield), creature spells you control (stack),
// creature cards you own that aren't on the battlefield — CR 613.4 layer 4.
static NEXUS_FILTER: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::Or(&[
        Filter::And(&[
            Filter::ControlledByYou,
            Filter::InZone(ZoneRef::Battlefield),
        ]),
        Filter::And(&[Filter::ControlledByYou, Filter::InZone(ZoneRef::Stack)]),
        Filter::And(&[Filter::OwnedByYou, Filter::InZone(ZoneRef::NotBattlefield)]),
    ]),
]);

use crate::tokens::SHAPESHIFTER_2_2_BLUE_CHANGELING as SHAPESHIFTER_TOKEN;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(92),
    oracle_id: "9b2cdbed-c733-409b-b0e4-2c8960c25111",
    scryfall_id: "1246c42d-57c0-4cba-959a-15ad89d8a50b",
    faces: &[FaceDef {
        name: "Maskwood Nexus",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Type,
            filter: NEXUS_FILTER,
            modifier: Modifier::AllCreatureTypes,
            cross_zone: true,
        }),
        AbilityDef::Activated {
            cost: Cost {
                mana: baylee_core::mana!("{3}"),
                parts: &[CostPart::TapSelf],
            },
            effects: &[Effect::CreateToken {
                token: &SHAPESHIFTER_TOKEN,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
    ..CardDef::DEFAULT
};

// Engine-level coverage lives in baylee-engine (m2 cross-zone test):
// with Nexus out, a non-Ally creature card in the library counts as an
// Ally for General Tazri's ETB tutor.
