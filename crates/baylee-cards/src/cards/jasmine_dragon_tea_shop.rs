//! Jasmine Dragon Tea Shop — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! {T}: Add one mana of any color. Spend this mana only to cast an Ally spell or activate an ability of an Ally source.
//! {5}, {T}: Create a 1/1 white Ally creature token.
//! Set: TLA #259 — Avatar: The Last Airbender | Scryfall ID: da2c83d4-a95f-47ff-a08f-694eb78d6b9b | Oracle ID: d9a24444-289f-473f-9985-8df275257555
// PARTIAL — the ally-restriction on the choice mana is not enforced yet
// (restricted mana riders land with the full payment solver, M2.S7+).
// Everything else implemented.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Effect, FaceDef, KeywordSet, PartnerKind, TokenDef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static ALLY_TOKEN: TokenDef = TokenDef {
    name: "Ally",
    colors: ColorSet::from_slice(&[Color::White]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::ALLY],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::EMPTY,
};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(77),
    oracle_id: "d9a24444-289f-473f-9985-8df275257555",
    scryfall_id: "da2c83d4-a95f-47ff-a08f-694eb78d6b9b",
    faces: &[FaceDef {
        name: "Jasmine Dragon Tea Shop",
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
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("ally-restricted mana rider (M2.S7+ payment solver)"),
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
            effects: &[Effect::AddManaChoice {
                colors: &[
                    ManaColor::White,
                    ManaColor::Blue,
                    ManaColor::Black,
                    ManaColor::Red,
                    ManaColor::Green,
                ],
                amount: baylee_cards_dsl::Amount::Fixed(1),
                combination: false,
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
};

#[cfg(test)]
mod tests {}
