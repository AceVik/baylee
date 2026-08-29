//! Bojuka Bog — (no cost) — Land
//! Oracle: Bojuka Bog enters the battlefield tapped.
//! When Bojuka Bog enters, exile all cards from target player's graveyard.
//! Set: C18 #259 — Commander 2018 | Scryfall ID: 55b5b094-9d2d-4d96-b90c-78fecdae725a | Oracle ID: 04b7362d-0490-4cb0-b5d7-2a7732f659ce
// IMPLEMENTED — ETB tapped + exile target player's graveyard (opponent
// auto-resolves heads-up; multiplayer player choice is a protocol M3 item).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Effect,
    EnterModifier, FaceDef, Filter, KeywordSet, PartnerKind, PlayerRel, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(14),
    oracle_id: "04b7362d-0490-4cb0-b5d7-2a7732f659ce",
    scryfall_id: "55b5b094-9d2d-4d96-b90c-78fecdae725a",
    faces: &[FaceDef {
        name: "Bojuka Bog",
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
        enter_modifiers: &[EnterModifier::Tapped],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::Black,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::ExileGraveyard {
                player: PlayerRel::Chosen,
            }],
            targets: Some(baylee_cards_dsl::TargetReq::one(
                baylee_cards_dsl::TargetSpec::AnyPlayer,
            )),
        },
    ],
};

#[cfg(test)]
mod tests {}
