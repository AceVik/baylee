//! Ashiok, Dream Render — {1}{U/B}{U/B} — Legendary Planeswalker — Ashiok
//! Oracle: Spells and abilities your opponents control can't cause their controller to search their library.
//! Oracle: −1: Target player mills four cards. Then exile each opponent's graveyard.
//! Set: WAR #228 — War of the Spark | Scryfall ID: f2df3258-c053-48a8-974f-d80899b2cd93 | Oracle ID: 93723b12-db34-4047-885e-8606415b1553
// IMPLEMENTED — search suppression (OpponentsCantSearch checked in the
// search ops) + the mill/exile-graveyards loyalty ability.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    Layer, Modifier, PartnerKind, PlayerRel, StaticAbility, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, planeswalker};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(8),
    oracle_id: "93723b12-db34-4047-885e-8606415b1553",
    scryfall_id: "f2df3258-c053-48a8-974f-d80899b2cd93",
    faces: &[FaceDef {
        name: "Ashiok, Dream Render",
        mana_cost: baylee_core::mana!("{1}{U/B}{U/B}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[planeswalker::ASHIOK],
        power: None,
        toughness: None,
        loyalty: Some(5),
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::Any,
            modifier: Modifier::OpponentsCantSearch,
            cross_zone: false,
        }),
        AbilityDef::Loyalty {
            cost: -1,
            effects: &[
                Effect::Mill {
                    amount: Amount::Fixed(4),
                    target: PlayerRel::ControllerOfTarget,
                },
                Effect::ExileGraveyard {
                    player: PlayerRel::EachOpponent,
                },
            ],
            target: Some(TargetSpec::AnyPlayer),
        },
    ],
};

#[cfg(test)]
mod tests {}
