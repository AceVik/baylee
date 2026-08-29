//! Archaeomancer — {2}{U}{U} — Creature — Human Wizard
//! Oracle: When this creature enters, return target instant or sorcery card from your graveyard to your hand.
//! Set: UMA #45 — Ultimate Masters | Scryfall ID: cc258713-6ce3-44e0-9b4b-8fa7d1d093a1 | Oracle ID: a91a3266-cadd-47a0-9b20-160307f14c07
// IMPLEMENTED — ETB spell recovery from your graveyard.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    PlayerRel, TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static SPELL_GY: Filter = Filter::Or(&[
    Filter::HasType(TypeSet::INSTANT),
    Filter::HasType(TypeSet::SORCERY),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(6),
    oracle_id: "a91a3266-cadd-47a0-9b20-160307f14c07",
    scryfall_id: "cc258713-6ce3-44e0-9b4b-8fa7d1d093a1",
    faces: &[FaceDef {
        name: "Archaeomancer",
        mana_cost: baylee_core::mana!("{2}{U}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::HUMAN, creature::WIZARD],
        power: Some(1),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&SPELL_GY, PlayerRel::You),
        }],
        targets: Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &SPELL_GY,
            PlayerRel::You,
        ))),
    }],
};

#[cfg(test)]
mod tests {}
