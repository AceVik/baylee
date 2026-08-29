//! Path to Exile — {W} — Instant
//! Oracle: Exile target creature. Its controller may search their library for a basic land card, put that card onto the battlefield tapped, then shuffle.
//! Set: MSC #141 — Marvel Super Heroes Commander | Scryfall ID: 95ca89ea-1200-4bb4-ae4b-af35d3ccd35b | Oracle ID: d683d985-9888-4d21-8b5f-69e69ce4a03b
// IMPLEMENTED — exile + optional basic-land ramp for the creature's controller.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    PlayerRel, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_F: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(112),
    oracle_id: "d683d985-9888-4d21-8b5f-69e69ce4a03b",
    scryfall_id: "95ca89ea-1200-4bb4-ae4b-af35d3ccd35b",
    faces: &[FaceDef {
        name: "Path to Exile",
        mana_cost: baylee_core::mana!("{W}"),
        types: TypeSet::INSTANT,
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
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[
            Effect::Exile {
                target: TargetSpec::Object(&CREATURE_F),
            },
            Effect::OptionalBasicLandSearchFor {
                player: PlayerRel::ControllerOfTarget,
            },
        ],
        targets: Some(TargetReq::one(TargetSpec::Object(&CREATURE_F))),
    }],
};

#[cfg(test)]
mod tests {}
