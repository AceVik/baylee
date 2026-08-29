//! Reanimate — {B} — Sorcery
//! Oracle: Put target creature card from a graveyard onto the battlefield under your control. You lose life equal to that card's mana value.
//! Set: DSC #155 — Duskmourn: House of Horror Commander | Scryfall ID: 368b6903-5fc4-43e7-bd44-46b8107c8bb4 | Oracle ID: a044474a-cd72-4e9d-bd8d-a08f2de9cdc0
// IMPLEMENTED — reanimation with life payment equal to cmc.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, PlayerRel, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_GY: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(125),
    oracle_id: "a044474a-cd72-4e9d-bd8d-a08f2de9cdc0",
    scryfall_id: "368b6903-5fc4-43e7-bd44-46b8107c8bb4",
    faces: &[FaceDef {
        name: "Reanimate",
        mana_cost: baylee_core::mana!("{B}"),
        types: TypeSet::SORCERY,
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[
            Effect::GraveyardToBattlefield {
                target: TargetSpec::CardInGraveyard(&CREATURE_GY, PlayerRel::EachPlayer),
            },
            Effect::LoseLife {
                amount: Amount::TargetCmc,
                target: PlayerRel::You,
            },
        ],
        targets: Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &CREATURE_GY,
            PlayerRel::EachPlayer,
        ))),
    }],
};

#[cfg(test)]
mod tests {}
