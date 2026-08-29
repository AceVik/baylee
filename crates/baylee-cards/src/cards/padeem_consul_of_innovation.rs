//! Padeem, Consul of Innovation — {3}{U} — Legendary Creature — Vedalken Artificer
//! Oracle: Artifacts you control have hexproof. (They can't be the targets of spells or abilities your opponents control.)
//! Oracle: At the beginning of your upkeep, if you control the artifact with the greatest mana value or tied for the greatest mana value, draw a card.
//! Set: CMM #109 — Commander Masters | Scryfall ID: 00a4aef8-64fc-4e9d-adac-ef4c85d40b4a | Oracle ID: 0c7ba712-6a99-4d2f-9242-a2163a11f69c
// IMPLEMENTED — hexproof grant + the greatest-cmc upkeep draw
// (IfControlGreatestCmc).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, Layer,
    Modifier, PartnerKind, StaticAbility, StepKind, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(108),
    oracle_id: "0c7ba712-6a99-4d2f-9242-a2163a11f69c",
    scryfall_id: "00a4aef8-64fc-4e9d-adac-ef4c85d40b4a",
    faces: &[FaceDef {
        name: "Padeem, Consul of Innovation",
        mana_cost: baylee_core::mana!("{3}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::VEDALKEN, creature::ARTIFICER],
        power: Some(1),
        toughness: Some(4),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: baylee_cards_dsl::PlayerRel::You,
            },
            once_per_turn: false,
            effects: &[Effect::IfControlGreatestCmc {
                filter: &Filter::HasType(TypeSet::ARTIFACT),
                then: &[Effect::DrawCards {
                    amount: baylee_cards_dsl::Amount::Fixed(1),
                }],
            }],
            targets: None,
        },
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: Filter::And(&[Filter::HasType(TypeSet::ARTIFACT), Filter::ControlledByYou]),
            modifier: Modifier::AddKeyword(KeywordSet::HEXPROOF),
            cross_zone: false,
        }),
    ],
};

#[cfg(test)]
mod tests {}
