//! Snapcaster Mage — {1}{U} — Creature — Human Wizard
//! Oracle: Flash
//! Oracle: When this creature enters, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (You may cast that card from your graveyard for its flashback cost. Then exile it.)
//! Set: INR #478 — Innistrad Remastered | Scryfall ID: 22b36ad5-bf4d-436a-9c3c-fa4acd0052fe | Oracle ID: 2bb2eda7-3b38-4c56-870f-c3218a1056f5
// IMPLEMENTED — flash + flashback grant until EOT (cast from graveyard,
// exile on resolution).
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

static INSTANT_OR_SORCERY: Filter = Filter::Or(&[
    Filter::HasType(TypeSet::INSTANT),
    Filter::HasType(TypeSet::SORCERY),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(148),
    oracle_id: "2bb2eda7-3b38-4c56-870f-c3218a1056f5",
    scryfall_id: "22b36ad5-bf4d-436a-9c3c-fa4acd0052fe",
    faces: &[FaceDef {
        name: "Snapcaster Mage",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::HUMAN, creature::WIZARD],
        power: Some(2),
        toughness: Some(1),
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::FLASH,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[Effect::GrantFlashback],
        targets: Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &INSTANT_OR_SORCERY,
            PlayerRel::You,
        ))),
    }],
};

#[cfg(test)]
mod tests {}
