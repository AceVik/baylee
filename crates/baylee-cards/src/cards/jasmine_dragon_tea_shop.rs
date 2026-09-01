//! Jasmine Dragon Tea Shop — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! {T}: Add one mana of any color. Spend this mana only to cast an Ally spell or activate an ability of an Ally source.
//! {5}, {T}: Create a 1/1 white Ally creature token.
//! Set: TLA #259 — Avatar: The Last Airbender | Scryfall ID: da2c83d4-a95f-47ff-a08f-694eb78d6b9b | Oracle ID: d9a24444-289f-473f-9985-8df275257555
// IMPLEMENTED — the ally-restriction on the choice mana is enforced via
// restricted mana provenance (spendable only on Ally spells). The "or
// activate an ability of an Ally source" half of the restriction is a
// payment-solver refinement (spells only today).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, TokenDef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

use crate::tokens::ALLY_1_1_WHITE as ALLY_TOKEN;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(77),
    oracle_id: "d9a24444-289f-473f-9985-8df275257555",
    scryfall_id: "da2c83d4-a95f-47ff-a08f-694eb78d6b9b",
    faces: &[FaceDef {
        name: "Jasmine Dragon Tea Shop",
        types: TypeSet::LAND,
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::Colorless,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddManaRestricted {
                colors: &[
                    ManaColor::White,
                    ManaColor::Blue,
                    ManaColor::Black,
                    ManaColor::Red,
                    ManaColor::Green,
                ],
                amount: 1,
                filter: &Filter::HasSubtype(creature::ALLY),
                rider: baylee_cards_dsl::SpendRider::None,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::Activated {
            cost: Cost {
                mana: baylee_core::mana!("{5}"),
                parts: &[CostPart::TapSelf],
            },
            effects: &[Effect::CreateToken { token: &ALLY_TOKEN }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
    ..CardDef::DEFAULT
};
