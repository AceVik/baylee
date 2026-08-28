//! Karmic Guide — {3}{W}{W} — Creature — Angel Spirit
//! Oracle: Flying, protection from black
//! Oracle: Echo {3}{W}{W} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)
//! Oracle: When this creature enters, return target creature card from your graveyard to the battlefield.
//! Set: SOC #151 — Secrets of Strixhaven Commander | Scryfall ID: b26d50dd-54a1-43ce-9884-3999f698d97b | Oracle ID: 8c31fec9-e4b3-4761-990e-7be38eb05604
// PARTIAL — flying + ETB reanimation implemented. NOT SUPPORTED yet: echo
// (delayed upkeep payment, M2.S7b) and protection from black (M2.S6+).
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

static CREATURE_GY: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(80),
    oracle_id: "8c31fec9-e4b3-4761-990e-7be38eb05604",
    scryfall_id: "b26d50dd-54a1-43ce-9884-3999f698d97b",
    faces: &[FaceDef {
        name: "Karmic Guide",
        mana_cost: baylee_core::mana!("{3}{W}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::ANGEL, creature::SPIRIT],
        power: Some(2),
        toughness: Some(2),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::FLYING,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("echo (M2.S7b), protection from black (M2.S6+)"),
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        effects: &[Effect::GraveyardToBattlefield {
            target: TargetSpec::CardInGraveyard(&CREATURE_GY, PlayerRel::You),
        }],
        targets: Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &CREATURE_GY,
            PlayerRel::You,
        ))),
    }],
};

#[cfg(test)]
mod tests {}
